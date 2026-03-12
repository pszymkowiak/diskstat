use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

#[derive(Serialize, Deserialize, Default)]
pub struct Config {
    #[serde(default)]
    pub scan: ScanConfig,
    #[serde(default)]
    pub ui: UiConfig,
    #[serde(default)]
    pub display: DisplayConfig,
}

#[derive(Serialize, Deserialize)]
pub struct ScanConfig {
    #[serde(default)]
    pub exclude: Vec<String>,
    #[serde(default = "default_max_nodes")]
    pub max_nodes: u64,
}

#[derive(Serialize, Deserialize)]
pub struct UiConfig {
    #[serde(default)]
    pub theme: usize,
    #[serde(default = "default_true")]
    pub show_treemap: bool,
    #[serde(default = "default_split_pct")]
    pub split_pct: u16,
}

#[derive(Serialize, Deserialize)]
pub struct DisplayConfig {
    #[serde(default = "default_top_files_count")]
    pub top_files_count: usize,
    #[serde(default)]
    pub sort_mode: String, // "size_desc", "size_asc", "name_asc", "name_desc", "age_newest", "age_oldest"
}

impl Default for ScanConfig {
    fn default() -> Self {
        ScanConfig {
            exclude: vec![],
            max_nodes: default_max_nodes(),
        }
    }
}

impl Default for UiConfig {
    fn default() -> Self {
        UiConfig {
            theme: 0,
            show_treemap: true,
            split_pct: default_split_pct(),
        }
    }
}

impl Default for DisplayConfig {
    fn default() -> Self {
        DisplayConfig {
            top_files_count: default_top_files_count(),
            sort_mode: "size_desc".to_string(),
        }
    }
}

fn default_max_nodes() -> u64 {
    10_000_000
}

fn default_true() -> bool {
    true
}

fn default_split_pct() -> u16 {
    40
}

fn default_top_files_count() -> usize {
    50
}

impl Config {
    /// Load config from ~/.config/diskstat/config.toml
    pub fn load() -> Self {
        let path = Self::config_path();
        if path.exists() {
            if let Ok(content) = fs::read_to_string(&path) {
                if let Ok(config) = toml::from_str(&content) {
                    return config;
                }
            }
        }
        Config::default()
    }

    /// Get the config file path
    pub fn config_path() -> PathBuf {
        let mut path = dirs::config_dir().unwrap_or_else(|| PathBuf::from("."));
        path.push("diskstat");
        path.push("config.toml");
        path
    }

    /// Save config to ~/.config/diskstat/config.toml
    pub fn save(&self) -> std::io::Result<()> {
        let path = Self::config_path();
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        let content = toml::to_string_pretty(self).unwrap();
        fs::write(path, content)?;
        Ok(())
    }
}

/// Helper to get dirs (fallback implementation if dirs crate is not available)
mod dirs {
    use std::env;
    use std::path::PathBuf;

    pub fn config_dir() -> Option<PathBuf> {
        #[cfg(target_os = "macos")]
        {
            env::var_os("HOME").map(|h| {
                let mut p = PathBuf::from(h);
                p.push("Library");
                p.push("Application Support");
                p
            })
        }
        #[cfg(not(target_os = "macos"))]
        {
            env::var_os("XDG_CONFIG_HOME")
                .map(PathBuf::from)
                .or_else(|| {
                    env::var_os("HOME").map(|h| {
                        let mut p = PathBuf::from(h);
                        p.push(".config");
                        p
                    })
                })
        }
    }
}
