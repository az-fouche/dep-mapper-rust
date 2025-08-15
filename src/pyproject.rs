use anyhow::Result;
use std::collections::HashSet;
use std::path::Path;
use std::sync::OnceLock;

/// Cached internal modules for the current project
static INTERNAL_MODULES_CACHE: OnceLock<(String, HashSet<String>)> = OnceLock::new();

/// Package mapping from pyproject.toml
#[derive(Debug, Clone)]
pub struct PackageMapping {
    pub include: String,
    pub from: Option<String>,
}

/// Parses pyproject.toml and extracts internal module names from packages section
pub fn get_internal_modules(project_root: &Path) -> Result<HashSet<String>> {
    let project_key = project_root.to_string_lossy().to_string();

    // Use cached result if available for the same project
    if let Some((cached_project, cached_modules)) = INTERNAL_MODULES_CACHE.get()
        && cached_project == &project_key
    {
        return Ok(cached_modules.clone());
    }

    let pyproject_path = project_root.join("pyproject.toml");

    if !pyproject_path.exists() {
        let empty_set = HashSet::new();
        // Only cache if we can set (first call)
        let _ = INTERNAL_MODULES_CACHE.set((project_key, empty_set.clone()));
        return Ok(empty_set);
    }

    let content = std::fs::read_to_string(&pyproject_path)?;
    let toml: toml::Value = toml::from_str(&content)?;

    let mut internal_modules = HashSet::new();

    if let Some(packages) = toml
        .get("tool")
        .and_then(|t| t.get("poetry"))
        .and_then(|p| p.get("packages"))
        .and_then(|p| p.as_array())
    {
        for package in packages {
            if let Some(include) = package.get("include").and_then(|i| i.as_str()) {
                internal_modules.insert(include.to_string());
            }
        }
    }

    // Cache the result for future calls to the same project
    // Only cache if we can set (first call)
    let _ = INTERNAL_MODULES_CACHE.set((project_key, internal_modules.clone()));

    Ok(internal_modules)
}

/// Gets package mappings from pyproject.toml
pub fn get_package_mappings(project_root: &Path) -> Result<Vec<PackageMapping>> {
    let pyproject_path = project_root.join("pyproject.toml");

    if !pyproject_path.exists() {
        return Ok(Vec::new());
    }

    let content = std::fs::read_to_string(&pyproject_path)?;
    let toml: toml::Value = toml::from_str(&content)?;

    let mut mappings = Vec::new();

    if let Some(packages) = toml
        .get("tool")
        .and_then(|t| t.get("poetry"))
        .and_then(|p| p.get("packages"))
        .and_then(|p| p.as_array())
    {
        for package in packages {
            if let Some(include) = package.get("include").and_then(|i| i.as_str()) {
                let from = package
                    .get("from")
                    .and_then(|f| f.as_str())
                    .map(|s| s.to_string());
                mappings.push(PackageMapping {
                    include: include.to_string(),
                    from,
                });
            }
        }
    }

    Ok(mappings)
}

/// Checks if a module name is internal based on the cached internal modules
pub fn is_internal_module(module_name: &str, project_root: &Path) -> bool {
    // Get internal modules (using cache if available)
    let internal_modules = match get_internal_modules(project_root) {
        Ok(modules) => modules,
        Err(_) => return false, // If we can't read pyproject.toml, assume external
    };

    // Check if the top-level module name is in our internal set
    let top_level = module_name.split('.').next().unwrap_or(module_name);
    internal_modules.contains(top_level)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn test_get_internal_modules() {
        let temp_dir = TempDir::new().unwrap();
        let pyproject_content = r#"
[tool.poetry]
packages = [
    { include = "common", from = "common/" },
    { include = "eva", from = "EVA/" },
]
"#;
        fs::write(temp_dir.path().join("pyproject.toml"), pyproject_content).unwrap();

        let internal_modules = get_internal_modules(temp_dir.path()).unwrap();
        assert_eq!(internal_modules.len(), 2);
        assert!(internal_modules.contains("common"));
        assert!(internal_modules.contains("eva"));
    }

    #[test]
    fn test_is_internal_module() {
        let temp_dir = TempDir::new().unwrap();
        let pyproject_content = r#"
[tool.poetry]
packages = [
    { include = "common", from = "common/" },
    { include = "eva", from = "EVA/" },
]
"#;
        fs::write(temp_dir.path().join("pyproject.toml"), pyproject_content).unwrap();

        assert!(is_internal_module("common", temp_dir.path()));
        assert!(is_internal_module("common.utils", temp_dir.path()));
        assert!(!is_internal_module("numpy", temp_dir.path()));
    }
}
