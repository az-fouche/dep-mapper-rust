use anyhow::Result;
use std::path::{Path, PathBuf};
use std::sync::OnceLock;

static PARSER: OnceLock<PyProjectParser> = OnceLock::new();

/// Package information from pyproject.toml
#[derive(Debug, Clone)]
pub struct PackageInfo {
    pub name: String,      // Python module name (e.g., "mymodule")
    pub directory: String, // Filesystem directory (e.g., "MyModule/")
}

/// Parser for pyproject.toml with project context
pub struct PyProjectParser {
    project_root: PathBuf,
    package_info: OnceLock<Vec<PackageInfo>>,
}

/// Filters out packages whose paths are contained within other packages' paths.
/// If module A's path is contained within module B's path, module A is ignored.
fn filter_contained_packages(mut packages: Vec<PackageInfo>) -> Vec<PackageInfo> {
    packages.sort_by(|a, b| a.directory.len().cmp(&b.directory.len()));

    let mut filtered = Vec::new();

    for package in packages {
        let is_contained = filtered.iter().any(|existing: &PackageInfo| {
            let existing_path = existing.directory.trim_end_matches('/');
            let package_path = package.directory.trim_end_matches('/');

            package_path.starts_with(&format!("{}/", existing_path))
                || (package_path.len() > existing_path.len()
                    && package_path.starts_with(existing_path))
        });

        if !is_contained {
            filtered.push(package);
        }
    }

    filtered
}

impl PyProjectParser {
    pub fn new(project_root: &Path) -> Self {
        Self {
            project_root: project_root.to_path_buf(),
            package_info: OnceLock::new(),
        }
    }

    fn load_package_info(&self) -> Result<Vec<PackageInfo>> {
        let pyproject_path = self.project_root.join("pyproject.toml");

        if !pyproject_path.exists() {
            return Ok(Vec::new());
        }

        let content = std::fs::read_to_string(&pyproject_path)?;
        let toml: toml::Value = toml::from_str(&content)?;

        let mut packages = Vec::new();

        if let Some(packages_array) = toml
            .get("tool")
            .and_then(|t| t.get("poetry"))
            .and_then(|p| p.get("packages"))
            .and_then(|p| p.as_array())
        {
            for package in packages_array {
                if let Some(include) = package.get("include").and_then(|i| i.as_str()) {
                    let directory = package
                        .get("from")
                        .and_then(|f| f.as_str())
                        .unwrap_or(include)
                        .to_string();

                    packages.push(PackageInfo {
                        name: include.to_string(),
                        directory,
                    });
                }
            }
        }

        Ok(filter_contained_packages(packages))
    }

    pub fn get_package_info(&self) -> &Vec<PackageInfo> {
        self.package_info
            .get_or_init(|| self.load_package_info().unwrap_or_default())
    }

    pub fn is_internal_module(&self, module_name: &str) -> bool {
        let packages = self.get_package_info();
        let top_level = module_name.split('.').next().unwrap_or(module_name);
        packages.iter().any(|pkg| pkg.name == top_level)
    }

    pub fn normalize_module_name(&self, module_name: &str) -> Result<String> {
        let packages = self.get_package_info();

        for package in packages {
            let from_dotted = package.directory.trim_end_matches('/').replace('/', ".");

            if module_name.starts_with(&format!("{}.", from_dotted)) {
                if let Some(remainder) = module_name.strip_prefix(&format!("{}.", from_dotted)) {
                    // Check if remainder already starts with the package name (common package/package/ structure)
                    if remainder.starts_with(&format!("{}.", package.name)) {
                        return Ok(remainder.to_string());
                    } else if remainder == package.name {
                        return Ok(package.name.clone());
                    } else {
                        return Ok(format!("{}.{}", package.name, remainder));
                    }
                } else if module_name == from_dotted {
                    return Ok(package.name.clone());
                }
            }
        }

        Ok(module_name.to_string())
    }
}

/// Initialize the module-level parser with project root
pub fn init(project_root: &Path) {
    PARSER.get_or_init(|| PyProjectParser::new(project_root));
}

pub fn is_internal_module(module_name: &str) -> bool {
    PARSER
        .get()
        .map_or(false, |parser| parser.is_internal_module(module_name))
}

pub fn normalize_module_name(module_name: &str) -> Result<String> {
    match PARSER.get() {
        Some(parser) => parser.normalize_module_name(module_name),
        None => Ok(module_name.to_string()),
    }
}

/// Computes the Python module name from file path relative to project root.
/// Uses pyproject.toml package definitions to normalize module names.
pub fn compute_module_name(file_path: &Path, project_root: &Path) -> Result<String> {
    let relative_path = file_path.strip_prefix(project_root).map_err(|_| {
        anyhow::anyhow!(
            "File path '{}' is not within project root '{}'",
            file_path.display(),
            project_root.display()
        )
    })?;

    let mut parts = Vec::new();

    // Add all directory components from the relative path
    for component in relative_path.components() {
        if let std::path::Component::Normal(name) = component
            && let Some(name_str) = name.to_str()
        {
            if name_str.ends_with(".py") {
                let file_stem = name_str.strip_suffix(".py").unwrap();
                if file_stem != "__init__" {
                    parts.push(file_stem.to_string());
                }
            } else {
                parts.push(name_str.to_string());
            }
        }
    }

    if parts.is_empty() {
        return Err(anyhow::anyhow!(
            "Could not determine module name from file path"
        ));
    }

    let full_name = parts.join(".");
    normalize_module_name(&full_name)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn test_get_package_info() {
        let temp_dir = TempDir::new().unwrap();
        let pyproject_content = r#"
[tool.poetry]
packages = [
    { include = "common", from = "common/" },
    { include = "mymodule", from = "MyModule/" },
]
"#;
        fs::write(temp_dir.path().join("pyproject.toml"), pyproject_content).unwrap();

        let parser = PyProjectParser::new(temp_dir.path());
        let packages = parser.get_package_info();
        assert_eq!(packages.len(), 2);

        let common = packages.iter().find(|p| p.name == "common").unwrap();
        assert_eq!(common.directory, "common/");

        let mymodule = packages.iter().find(|p| p.name == "mymodule").unwrap();
        assert_eq!(mymodule.directory, "MyModule/");
    }

    #[test]
    fn test_is_internal_module() {
        let temp_dir = TempDir::new().unwrap();
        let pyproject_content = r#"
[tool.poetry]
packages = [
    { include = "common", from = "common/" },
    { include = "mymodule", from = "MyModule/" },
]
"#;
        fs::write(temp_dir.path().join("pyproject.toml"), pyproject_content).unwrap();

        // Create a direct parser instance for this test to avoid global state
        let parser = PyProjectParser::new(temp_dir.path());

        assert!(parser.is_internal_module("common"));
        assert!(parser.is_internal_module("common.utils"));
        assert!(!parser.is_internal_module("numpy"));
    }

    #[test]
    fn test_filter_contained_packages() {
        let packages = vec![
            PackageInfo {
                name: "medcat".to_string(),
                directory: "ehr_data_formatter/medcat/".to_string(),
            },
            PackageInfo {
                name: "ehr_data_formatter".to_string(),
                directory: "ehr_data_formatter/".to_string(),
            },
            PackageInfo {
                name: "other".to_string(),
                directory: "other/".to_string(),
            },
        ];

        let filtered = filter_contained_packages(packages);

        assert_eq!(filtered.len(), 2);
        assert!(filtered.iter().any(|p| p.name == "ehr_data_formatter"));
        assert!(filtered.iter().any(|p| p.name == "other"));
        assert!(!filtered.iter().any(|p| p.name == "medcat"));
    }

    #[test]
    fn test_compute_module_name() {
        let temp_dir = TempDir::new().unwrap();
        init(temp_dir.path());

        let project_root = temp_dir.path();

        // Test simple file
        let file_path = project_root.join("main.py");
        fs::write(&file_path, "").unwrap();
        assert_eq!(
            compute_module_name(&file_path, project_root).unwrap(),
            "main"
        );

        // Test package module
        fs::create_dir_all(project_root.join("package")).unwrap();
        let file_path = project_root.join("package/module.py");
        fs::write(&file_path, "").unwrap();
        assert_eq!(
            compute_module_name(&file_path, project_root).unwrap(),
            "package.module"
        );

        // Test __init__.py
        let file_path = project_root.join("package/__init__.py");
        fs::write(&file_path, "").unwrap();
        assert_eq!(
            compute_module_name(&file_path, project_root).unwrap(),
            "package"
        );
    }
}
