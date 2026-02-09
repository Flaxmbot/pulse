//! Pulse Package Manager
//! 
//! Handles package manifests, dependency resolution, and package operations.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

/// Package manifest (Pulse.toml)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Manifest {
    pub package: Package,
    #[serde(default)]
    pub dependencies: HashMap<String, Dependency>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Package {
    pub name: String,
    pub version: String,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub authors: Vec<String>,
    #[serde(default)]
    pub license: Option<String>,
    #[serde(default = "default_entry")]
    pub entry: String,
}

fn default_entry() -> String {
    "src/main.pulse".to_string()
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum Dependency {
    Version(String),
    Detailed(DetailedDependency),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DetailedDependency {
    pub version: Option<String>,
    pub path: Option<String>,
    pub git: Option<String>,
    pub branch: Option<String>,
}

impl Manifest {
    /// Load manifest from Pulse.toml in the given directory
    pub fn load(dir: &Path) -> Result<Self, String> {
        let manifest_path = dir.join("Pulse.toml");
        let content = fs::read_to_string(&manifest_path)
            .map_err(|e| format!("Failed to read Pulse.toml: {}", e))?;
        toml::from_str(&content)
            .map_err(|e| format!("Failed to parse Pulse.toml: {}", e))
    }
    
    /// Save manifest to Pulse.toml
    pub fn save(&self, dir: &Path) -> Result<(), String> {
        let manifest_path = dir.join("Pulse.toml");
        let content = toml::to_string_pretty(self)
            .map_err(|e| format!("Failed to serialize manifest: {}", e))?;
        fs::write(&manifest_path, content)
            .map_err(|e| format!("Failed to write Pulse.toml: {}", e))
    }
    
    /// Create a new manifest with defaults
    pub fn new(name: String) -> Self {
        Self {
            package: Package {
                name,
                version: "0.1.0".to_string(),
                description: None,
                authors: Vec::new(),
                license: None,
                entry: default_entry(),
            },
            dependencies: HashMap::new(),
        }
    }
}

/// Initialize a new Pulse project
pub fn init_project(dir: &Path, name: Option<String>) -> Result<(), String> {
    let project_name = name.unwrap_or_else(|| {
        dir.file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("pulse_project")
            .to_string()
    });
    
    // Create Pulse.toml
    let manifest = Manifest::new(project_name.clone());
    manifest.save(dir)?;
    
    // Create src directory
    let src_dir = dir.join("src");
    fs::create_dir_all(&src_dir)
        .map_err(|e| format!("Failed to create src directory: {}", e))?;
    
    // Create main.pulse
    let main_file = src_dir.join("main.pulse");
    if !main_file.exists() {
        fs::write(&main_file, format!(r#"// {} - A Pulse project
// Entry point

println("Hello from {}!");
"#, project_name, project_name))
            .map_err(|e| format!("Failed to create main.pulse: {}", e))?;
    }
    
    // Create .gitignore
    let gitignore = dir.join(".gitignore");
    if !gitignore.exists() {
        fs::write(&gitignore, "/target\n/.pulse_cache\n")
            .map_err(|e| format!("Failed to create .gitignore: {}", e))?;
    }
    
    Ok(())
}

/// Add a dependency to the project
pub fn add_dependency(dir: &Path, name: &str, version: Option<&str>) -> Result<(), String> {
    let mut manifest = Manifest::load(dir)?;
    
    let dep = match version {
        Some(v) => Dependency::Version(v.to_string()),
        None => Dependency::Version("*".to_string()),
    };
    
    manifest.dependencies.insert(name.to_string(), dep);
    manifest.save(dir)?;
    
    Ok(())
}

/// Get the packages directory (for future use)
#[allow(dead_code)]
pub fn packages_dir() -> PathBuf {
    dirs::home_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join(".pulse")
        .join("packages")
}
