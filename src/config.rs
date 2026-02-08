//! Configuration management for SQL TUI
//! 
//! Handles loading and saving connection configurations to ~/.config/sqltui/config.json

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

/// Configuration for a single database connection
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ConnectionConfig {
    pub name: String,
    pub host: String,
    pub port: u16,
    pub user: String,
    pub password: String,
    pub database: String,
}

impl Default for ConnectionConfig {
    fn default() -> Self {
        Self {
            name: String::new(),
            host: String::new(),
            port: 1433,
            user: String::new(),
            password: String::new(),
            database: "master".to_string(),
        }
    }
}

impl ConnectionConfig {
    /// Check if all required fields are filled
    pub fn is_valid(&self) -> bool {
        !self.name.trim().is_empty()
            && !self.host.trim().is_empty()
            && self.port > 0
            && !self.user.trim().is_empty()
            // password can be empty for Windows Auth
            && !self.database.trim().is_empty()
    }
}

/// Application configuration
#[derive(Serialize, Deserialize, Default, Clone, Debug)]
pub struct AppConfig {
    /// List of saved connections
    pub connections: Vec<ConnectionConfig>,
    /// Name of the last used connection (for auto-connect)
    pub last_connection: Option<String>,
    /// Locale override (e.g., "pt-BR", "en"). If None, uses system locale
    #[serde(default)]
    pub locale: Option<String>,
}

impl AppConfig {
    /// Get the config file path (~/.config/sqltui/config.json)
    fn config_path() -> Result<PathBuf> {
        let config_dir = dirs::config_dir()
            .context("Could not find config directory")?
            .join("sqltui");
        
        Ok(config_dir.join("config.json"))
    }

    /// Load configuration from disk, creating empty config if it doesn't exist
    pub fn load() -> Self {
        match Self::try_load() {
            Ok(config) => config,
            Err(_) => {
                let config = Self::default();
                // Try to save the empty config
                let _ = config.save();
                config
            }
        }
    }

    /// Try to load configuration from disk
    fn try_load() -> Result<Self> {
        let path = Self::config_path()?;
        
        if !path.exists() {
            return Ok(Self::default());
        }

        let contents = fs::read_to_string(&path)
            .context("Failed to read config file")?;
        
        let config: Self = serde_json::from_str(&contents)
            .context("Failed to parse config file")?;
        
        Ok(config)
    }

    /// Save configuration to disk
    pub fn save(&self) -> Result<()> {
        let path = Self::config_path()?;
        
        // Create directory if it doesn't exist
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)
                .context("Failed to create config directory")?;
        }

        let contents = serde_json::to_string_pretty(self)
            .context("Failed to serialize config")?;
        
        fs::write(&path, contents)
            .context("Failed to write config file")?;
        
        Ok(())
    }

    /// Add or update a connection (updates if name already exists)
    pub fn add_connection(&mut self, conn: ConnectionConfig) {
        if let Some(existing) = self.connections.iter_mut().find(|c| c.name == conn.name) {
            *existing = conn;
        } else {
            self.connections.push(conn);
        }
    }

    /// Remove a connection by name
    pub fn remove_connection(&mut self, name: &str) {
        self.connections.retain(|c| c.name != name);
        
        // Clear last_connection if it was the removed one
        if self.last_connection.as_deref() == Some(name) {
            self.last_connection = None;
        }
    }

    /// Get a connection by name
    pub fn get_connection(&self, name: &str) -> Option<&ConnectionConfig> {
        self.connections.iter().find(|c| c.name == name)
    }

    /// Set the last used connection
    pub fn set_last_connection(&mut self, name: &str) {
        self.last_connection = Some(name.to_string());
    }
}

/// Form state for editing a connection
#[derive(Clone, Debug, Default)]
pub struct ConnectionForm {
    pub name: String,
    pub host: String,
    pub port: String,
    pub user: String,
    pub password: String,
    pub database: String,
    pub is_new: bool,
}

impl ConnectionForm {
    /// Create a new empty form for creating a connection
    pub fn new_empty() -> Self {
        Self {
            name: String::new(),
            host: String::new(),
            port: "1433".to_string(),
            user: String::new(),
            password: String::new(),
            database: "master".to_string(),
            is_new: true,
        }
    }

    /// Create a form from an existing connection config
    pub fn from_config(config: &ConnectionConfig) -> Self {
        Self {
            name: config.name.clone(),
            host: config.host.clone(),
            port: config.port.to_string(),
            user: config.user.clone(),
            password: config.password.clone(),
            database: config.database.clone(),
            is_new: false,
        }
    }

    /// Convert form to ConnectionConfig
    pub fn to_config(&self) -> Option<ConnectionConfig> {
        let port: u16 = self.port.parse().ok()?;
        
        let config = ConnectionConfig {
            name: self.name.trim().to_string(),
            host: self.host.trim().to_string(),
            port,
            user: self.user.trim().to_string(),
            password: self.password.clone(),
            database: self.database.trim().to_string(),
        };

        if config.is_valid() {
            Some(config)
        } else {
            None
        }
    }

    /// Check if the form has all required fields filled
    pub fn is_valid(&self) -> bool {
        self.to_config().is_some()
    }

    /// Get field value by index (0-5)
    pub fn get_field(&self, index: usize) -> &str {
        match index {
            0 => &self.name,
            1 => &self.host,
            2 => &self.port,
            3 => &self.user,
            4 => &self.password,
            5 => &self.database,
            _ => "",
        }
    }

    /// Get mutable field value by index (0-5)
    pub fn get_field_mut(&mut self, index: usize) -> Option<&mut String> {
        match index {
            0 => Some(&mut self.name),
            1 => Some(&mut self.host),
            2 => Some(&mut self.port),
            3 => Some(&mut self.user),
            4 => Some(&mut self.password),
            5 => Some(&mut self.database),
            _ => None,
        }
    }

    /// Get field label by index
    pub fn get_field_label(index: usize) -> &'static str {
        match index {
            0 => "Nome",
            1 => "Host",
            2 => "Porta",
            3 => "User Id",
            4 => "Password",
            5 => "Database",
            _ => "",
        }
    }

    /// Total number of fields
    pub const FIELD_COUNT: usize = 6;
}
