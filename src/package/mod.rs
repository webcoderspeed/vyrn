use std::collections::HashMap;

/// Represents a Vryn package with metadata and dependencies
#[derive(Debug, Clone, PartialEq)]
#[allow(dead_code)]
pub struct VrynPackage {
    pub name: String,
    pub version: String,
    pub description: Option<String>,
    pub author: Option<String>,
    pub edition: Option<String>,
    pub dependencies: HashMap<String, String>,
}

impl VrynPackage {
    /// Create a new VrynPackage with given name and version
    #[allow(dead_code)]
    pub fn new(name: String, version: String) -> Self {
        VrynPackage {
            name,
            version,
            description: None,
            author: None,
            edition: None,
            dependencies: HashMap::new(),
        }
    }
}

/// Parse a vryn.toml format string into a VrynPackage
#[allow(dead_code)]
pub fn parse_vryn_toml(content: &str) -> Result<VrynPackage, String> {
    let mut package = VrynPackage::new("unknown".to_string(), "0.0.0".to_string());
    let mut in_package_section = false;
    let mut in_dependencies_section = false;

    for line in content.lines() {
        let trimmed = line.trim();

        // Skip empty lines and comments
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }

        // Check for section headers
        if trimmed == "[package]" {
            in_package_section = true;
            in_dependencies_section = false;
            continue;
        }

        if trimmed == "[dependencies]" {
            in_package_section = false;
            in_dependencies_section = true;
            continue;
        }

        // Skip other section headers
        if trimmed.starts_with('[') && trimmed.ends_with(']') {
            in_package_section = false;
            in_dependencies_section = false;
            continue;
        }

        // Parse key = value pairs
        if let Some(eq_pos) = trimmed.find('=') {
            let key = trimmed[..eq_pos].trim();
            let value_part = trimmed[eq_pos + 1..].trim();

            // Remove surrounding quotes if present
            let value = if (value_part.starts_with('"') && value_part.ends_with('"')) ||
                           (value_part.starts_with('\'') && value_part.ends_with('\'')) {
                value_part[1..value_part.len() - 1].to_string()
            } else {
                value_part.to_string()
            };

            if in_package_section {
                match key {
                    "name" => package.name = value,
                    "version" => package.version = value,
                    "description" => package.description = Some(value),
                    "author" => package.author = Some(value),
                    "edition" => package.edition = Some(value),
                    _ => {
                        return Err(format!("Unknown package field: {}", key));
                    }
                }
            } else if in_dependencies_section {
                package.dependencies.insert(key.to_string(), value);
            }
        }
    }

    // Validate that name and version are set
    if package.name == "unknown" {
        return Err("Package name not specified".to_string());
    }
    if package.version == "0.0.0" {
        return Err("Package version not specified".to_string());
    }

    Ok(package)
}

/// Generate a vryn.toml formatted string from a VrynPackage
#[allow(dead_code)]
pub fn generate_vryn_toml(package: &VrynPackage) -> String {
    let mut toml = String::from("[package]\n");
    toml.push_str(&format!("name = \"{}\"\n", package.name));
    toml.push_str(&format!("version = \"{}\"\n", package.version));

    if let Some(desc) = &package.description {
        toml.push_str(&format!("description = \"{}\"\n", desc));
    }

    if let Some(author) = &package.author {
        toml.push_str(&format!("author = \"{}\"\n", author));
    }

    if let Some(edition) = &package.edition {
        toml.push_str(&format!("edition = \"{}\"\n", edition));
    }

    // Add dependencies section if there are any
    if !package.dependencies.is_empty() {
        toml.push_str("\n[dependencies]\n");
        let mut deps: Vec<_> = package.dependencies.iter().collect();
        deps.sort_by_key(|(k, _)| *k);
        for (name, version) in deps {
            toml.push_str(&format!("{} = \"{}\"\n", name, version));
        }
    }

    toml
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_vryn_package_creation() {
        let pkg = VrynPackage::new("my-project".to_string(), "0.1.0".to_string());
        assert_eq!(pkg.name, "my-project");
        assert_eq!(pkg.version, "0.1.0");
        assert_eq!(pkg.description, None);
        assert_eq!(pkg.author, None);
        assert!(pkg.dependencies.is_empty());
    }

    #[test]
    fn test_parse_basic_toml() {
        let content = r#"[package]
name = "hello-world"
version = "1.0.0"
"#;
        let pkg = parse_vryn_toml(content).unwrap();
        assert_eq!(pkg.name, "hello-world");
        assert_eq!(pkg.version, "1.0.0");
        assert_eq!(pkg.description, None);
        assert_eq!(pkg.author, None);
    }

    #[test]
    fn test_parse_toml_with_all_fields() {
        let content = r#"[package]
name = "my-project"
version = "0.1.0"
description = "A Vryn project"
author = "Sanjeev Sharma"
edition = "2024"
"#;
        let pkg = parse_vryn_toml(content).unwrap();
        assert_eq!(pkg.name, "my-project");
        assert_eq!(pkg.version, "0.1.0");
        assert_eq!(pkg.description, Some("A Vryn project".to_string()));
        assert_eq!(pkg.author, Some("Sanjeev Sharma".to_string()));
        assert_eq!(pkg.edition, Some("2024".to_string()));
    }

    #[test]
    fn test_parse_toml_with_dependencies() {
        let content = r#"[package]
name = "my-project"
version = "0.1.0"

[dependencies]
"#;
        let pkg = parse_vryn_toml(content).unwrap();
        assert_eq!(pkg.name, "my-project");
        assert!(pkg.dependencies.is_empty());
    }

    #[test]
    fn test_parse_toml_missing_name() {
        let content = r#"[package]
version = "1.0.0"
"#;
        let result = parse_vryn_toml(content);
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), "Package name not specified");
    }

    #[test]
    fn test_parse_toml_missing_version() {
        let content = r#"[package]
name = "my-project"
"#;
        let result = parse_vryn_toml(content);
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), "Package version not specified");
    }

    #[test]
    fn test_generate_basic_toml() {
        let pkg = VrynPackage::new("test-pkg".to_string(), "2.0.0".to_string());
        let toml = generate_vryn_toml(&pkg);
        
        assert!(toml.contains("[package]"));
        assert!(toml.contains(r#"name = "test-pkg""#));
        assert!(toml.contains(r#"version = "2.0.0""#));
    }

    #[test]
    fn test_generate_toml_with_all_fields() {
        let mut pkg = VrynPackage::new("my-app".to_string(), "1.5.0".to_string());
        pkg.description = Some("A test application".to_string());
        pkg.author = Some("John Doe".to_string());
        pkg.edition = Some("2025".to_string());

        let toml = generate_vryn_toml(&pkg);
        
        assert!(toml.contains(r#"name = "my-app""#));
        assert!(toml.contains(r#"version = "1.5.0""#));
        assert!(toml.contains(r#"description = "A test application""#));
        assert!(toml.contains(r#"author = "John Doe""#));
        assert!(toml.contains(r#"edition = "2025""#));
    }

    #[test]
    fn test_roundtrip_parse_and_generate() {
        let content = r#"[package]
name = "roundtrip-test"
version = "3.2.1"
description = "Testing roundtrip"
author = "Test Author"
edition = "2024"
"#;
        let pkg = parse_vryn_toml(content).unwrap();
        let generated = generate_vryn_toml(&pkg);
        
        // Parse the generated TOML and verify it matches
        let pkg2 = parse_vryn_toml(&generated).unwrap();
        assert_eq!(pkg, pkg2);
    }
}
