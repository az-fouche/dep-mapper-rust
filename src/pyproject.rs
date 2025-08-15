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

        Ok(packages)
    }

    pub fn get_package_info(&self) -> &Vec<PackageInfo> {
        self.package_info.get_or_init(|| {
            self.load_package_info().unwrap_or_default()
        })
    }

    pub fn is_internal_module(&self, module_name: &str) -> bool {
        let packages = self.get_package_info();
        let top_level = module_name.split('.').next().unwrap_or(module_name);
        packages.iter().any(|pkg| pkg.name == top_level)
    }

    pub fn normalize_module_name(&self, module_name: &str) -> Result<String> {
        let packages = self.get_package_info();

        for package in packages {
            let from_dotted = package.directory.replace('/', ".");

            if module_name.starts_with(&from_dotted) {
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
    PARSER.get().map_or(false, |parser| parser.is_internal_module(module_name))
}

pub fn normalize_module_name(module_name: &str) -> Result<String> {
    match PARSER.get() {
        Some(parser) => parser.normalize_module_name(module_name),
        None => Ok(module_name.to_string()),
    }
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

        // Initialize the parser for this test
        init(temp_dir.path());
        
        assert!(is_internal_module("common"));
        assert!(is_internal_module("common.utils"));
        assert!(!is_internal_module("numpy"));
    }
}
