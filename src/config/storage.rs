use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct DirectoryItem {
    pub path: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct DirectoriesConfig {
    directories: DirectoriesTable,
    #[serde(default, rename = "Cache")]
    cache: Option<CacheTable>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct DirectoriesTable {
    path: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize)]
struct CacheTable {
    #[serde(default = "default_cache_dir")]
    directory: String,
}

pub fn default_cache_dir() -> String {
    dirs::data_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("spessartine")
        .join("thumbnails")
        .to_string_lossy()
        .to_string()
}

impl DirectoriesConfig {
    pub fn new() -> Self {
        let mut config = Self {
            directories: DirectoriesTable { path: Vec::new() },
            cache: None,
        };
        config.directories.path.push(
            super::base_dir().join("assets/example").to_string_lossy().to_string()
        );
        config
    }

    pub fn load() -> Self {
        let path = super::base_dir().join("storage.toml");
        if let Ok(content) = fs::read_to_string(&path) {
            if let Ok(config) = toml::from_str(&content) {
                return config;
            }
        }
        Self::new()
    }

    pub fn save(&self) -> std::io::Result<()> {
        let base = super::base_dir();
        let dir = &base;
        if !dir.exists() {
            fs::create_dir_all(dir)?;
        }
        let toml_string = toml::to_string_pretty(self).expect("Failed to serialize config to TOML");
        fs::write(base.join("storage.toml"), toml_string)?;
        Ok(())
    }

    pub fn add_directory(&mut self, path:String) {
        if !self.directories.path.contains(&path) {
            self.directories.path.push(path);
        }
    }
    
    pub fn remove_directory(&mut self, index: usize) {
        if index < self.directories.path.len() {
            self.directories.path.remove(index);
        }
    }

    pub fn get_directories(&self) -> Vec<DirectoryItem> {
        self.directories.path.iter()
            .map(|p| DirectoryItem { path: p.clone() })
            .collect()
    }

    pub fn get_cache_dir(&self) -> String {
        self.cache.as_ref()
            .map(|c| c.directory.clone())
            .unwrap_or_else(default_cache_dir)
    }

    pub fn set_cache_dir(&mut self, path: String) {
        self.cache = Some(CacheTable { directory: path });
    }
}
