//! Configuration management for SQL TUI
//!
//! Handles loading and saving connection configurations to ~/.config/sqltui/config.json

use crate::db::DatabaseBackend;
use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

/// Configuration for a single database connection
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ConnectionConfig {
    pub name: String,
    /// Which backend this connection uses
    #[serde(default)]
    pub backend: DatabaseBackend,
    // --- SQL Server fields ---
    #[serde(default)]
    pub host: String,
    #[serde(default = "default_port")]
    pub port: u16,
    #[serde(default)]
    pub user: String,
    #[serde(default)]
    pub password: String,
    #[serde(default = "default_database")]
    pub database: String,
    // --- SQLite fields ---
    /// Path to the SQLite .db file (only used when backend == Sqlite)
    #[serde(default)]
    pub sqlite_path: String,
}

fn default_port() -> u16 { 1433 }
fn default_database() -> String { "master".to_string() }

impl Default for ConnectionConfig {
    fn default() -> Self {
        Self {
            name: String::new(),
            backend: DatabaseBackend::SqlServer,
            host: String::new(),
            port: 1433,
            user: String::new(),
            password: String::new(),
            database: "master".to_string(),
            sqlite_path: String::new(),
        }
    }
}

impl ConnectionConfig {
    /// Check if all required fields are filled
    pub fn is_valid(&self) -> bool {
        if self.name.trim().is_empty() {
            return false;
        }
        match self.backend {
            DatabaseBackend::SqlServer => {
                !self.host.trim().is_empty()
                    && self.port > 0
                    && !self.user.trim().is_empty()
                    && !self.database.trim().is_empty()
            }
            DatabaseBackend::Sqlite => {
                !self.sqlite_path.trim().is_empty()
            }
        }
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
                let _ = config.save();
                config
            }
        }
    }

    fn try_load() -> Result<Self> {
        let path = Self::config_path()?;
        if !path.exists() {
            return Ok(Self::default());
        }
        let contents = fs::read_to_string(&path).context("Failed to read config file")?;
        let config: Self = serde_json::from_str(&contents).context("Failed to parse config file")?;
        Ok(config)
    }

    pub fn save(&self) -> Result<()> {
        let path = Self::config_path()?;
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).context("Failed to create config directory")?;
        }
        let contents = serde_json::to_string_pretty(self).context("Failed to serialize config")?;
        fs::write(&path, contents).context("Failed to write config file")?;
        Ok(())
    }

    pub fn add_connection(&mut self, conn: ConnectionConfig) {
        if let Some(existing) = self.connections.iter_mut().find(|c| c.name == conn.name) {
            *existing = conn;
        } else {
            self.connections.push(conn);
        }
    }

    pub fn remove_connection(&mut self, name: &str) {
        self.connections.retain(|c| c.name != name);
        if self.last_connection.as_deref() == Some(name) {
            self.last_connection = None;
        }
    }

    pub fn get_connection(&self, name: &str) -> Option<&ConnectionConfig> {
        self.connections.iter().find(|c| c.name == name)
    }

    pub fn set_last_connection(&mut self, name: &str) {
        self.last_connection = Some(name.to_string());
    }
}

/// Form state for editing a connection
#[derive(Clone, Debug, Default)]
pub struct ConnectionForm {
    pub backend: DatabaseBackend,
    pub name: String,
    // SQL Server fields
    pub host: String,
    pub port: String,
    pub user: String,
    pub password: String,
    pub database: String,
    // SQLite fields
    pub sqlite_path: String,
    pub is_new: bool,
}

impl ConnectionForm {
    /// Create a new empty form for creating a connection
    pub fn new_empty() -> Self {
        Self {
            backend: DatabaseBackend::SqlServer,
            name: String::new(),
            host: String::new(),
            port: "1433".to_string(),
            user: String::new(),
            password: String::new(),
            database: "master".to_string(),
            sqlite_path: String::new(),
            is_new: true,
        }
    }

    /// Create a form from an existing connection config
    pub fn from_config(config: &ConnectionConfig) -> Self {
        Self {
            backend: config.backend,
            name: config.name.clone(),
            host: config.host.clone(),
            port: config.port.to_string(),
            user: config.user.clone(),
            password: config.password.clone(),
            database: config.database.clone(),
            sqlite_path: config.sqlite_path.clone(),
            is_new: false,
        }
    }

    /// Convert form to ConnectionConfig
    pub fn to_config(&self) -> Option<ConnectionConfig> {
        let config = match self.backend {
            DatabaseBackend::SqlServer => {
                let port: u16 = self.port.parse().ok()?;
                ConnectionConfig {
                    name: self.name.trim().to_string(),
                    backend: DatabaseBackend::SqlServer,
                    host: self.host.trim().to_string(),
                    port,
                    user: self.user.trim().to_string(),
                    password: self.password.clone(),
                    database: self.database.trim().to_string(),
                    sqlite_path: String::new(),
                }
            }
            DatabaseBackend::Sqlite => {
                ConnectionConfig {
                    name: self.name.trim().to_string(),
                    backend: DatabaseBackend::Sqlite,
                    host: String::new(),
                    port: 0,
                    user: String::new(),
                    password: String::new(),
                    database: String::new(),
                    sqlite_path: self.sqlite_path.trim().to_string(),
                }
            }
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

    /// Total number of visible fields (depends on backend)
    pub fn field_count(&self) -> usize {
        match self.backend {
            DatabaseBackend::SqlServer => 6,  // name, host, port, user, password, database
            DatabaseBackend::Sqlite => 2,      // name, sqlite_path
        }
    }

    /// FIELD_COUNT is kept for backward compat with the SQL Server max
    pub const FIELD_COUNT: usize = 6;

    /// Get field value by index
    pub fn get_field(&self, index: usize) -> &str {
        match self.backend {
            DatabaseBackend::SqlServer => match index {
                0 => &self.name,
                1 => &self.host,
                2 => &self.port,
                3 => &self.user,
                4 => &self.password,
                5 => &self.database,
                _ => "",
            },
            DatabaseBackend::Sqlite => match index {
                0 => &self.name,
                1 => &self.sqlite_path,
                _ => "",
            },
        }
    }

    /// Get mutable field value by index
    pub fn get_field_mut(&mut self, index: usize) -> Option<&mut String> {
        match self.backend {
            DatabaseBackend::SqlServer => match index {
                0 => Some(&mut self.name),
                1 => Some(&mut self.host),
                2 => Some(&mut self.port),
                3 => Some(&mut self.user),
                4 => Some(&mut self.password),
                5 => Some(&mut self.database),
                _ => None,
            },
            DatabaseBackend::Sqlite => match index {
                0 => Some(&mut self.name),
                1 => Some(&mut self.sqlite_path),
                _ => None,
            },
        }
    }

    /// Get field label by index
    pub fn get_field_label(&self, index: usize) -> &'static str {
        match self.backend {
            DatabaseBackend::SqlServer => match index {
                0 => "Nome",
                1 => "Host",
                2 => "Porta",
                3 => "User Id",
                4 => "Password",
                5 => "Database",
                _ => "",
            },
            DatabaseBackend::Sqlite => match index {
                0 => "Nome",
                1 => "Arquivo",
                _ => "",
            },
        }
    }

    /// Is this field a password field?
    pub fn is_password_field(&self, index: usize) -> bool {
        matches!(self.backend, DatabaseBackend::SqlServer) && index == 4
    }
}
