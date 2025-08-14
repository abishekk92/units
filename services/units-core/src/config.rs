use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::path::Path;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub storage: StorageConfig,
    pub runtime: RuntimeConfig,
    pub server: ServerConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StorageConfig {
    /// Storage type: "memory" or "file"
    pub storage_type: String,
    /// Data directory for file-based storage
    pub data_dir: Option<String>,
    /// Maximum object size in bytes
    pub max_object_size: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RuntimeConfig {
    /// Maximum VM execution time in milliseconds
    pub max_execution_time_ms: u64,
    /// Maximum VM memory usage in bytes
    pub max_memory_bytes: usize,
    /// Maximum VM instruction count
    pub max_instructions: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerConfig {
    /// Maximum concurrent connections
    pub max_connections: u32,
    /// Request timeout in seconds
    pub request_timeout_secs: u64,
    /// Enable CORS for web clients
    pub enable_cors: bool,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            storage: StorageConfig {
                storage_type: "memory".to_string(),
                data_dir: None,
                max_object_size: 10 * 1024 * 1024, // 10MB
            },
            runtime: RuntimeConfig {
                max_execution_time_ms: 5000, // 5 seconds
                max_memory_bytes: 64 * 1024 * 1024, // 64MB
                max_instructions: 1_000_000,
            },
            server: ServerConfig {
                max_connections: 1000,
                request_timeout_secs: 30,
                enable_cors: true,
            },
        }
    }
}

impl Config {
    pub fn load<P: AsRef<Path>>(path: P) -> Result<Self> {
        let path = path.as_ref();
        if path.exists() {
            let content = std::fs::read_to_string(path)?;
            let config: Config = toml::from_str(&content)?;
            Ok(config)
        } else {
            // Create default config file
            let config = Config::default();
            let content = toml::to_string_pretty(&config)?;
            std::fs::write(path, content)?;
            Ok(config)
        }
    }

    #[allow(dead_code)]
    pub fn save<P: AsRef<Path>>(&self, path: P) -> Result<()> {
        let content = toml::to_string_pretty(self)?;
        std::fs::write(path, content)?;
        Ok(())
    }
}