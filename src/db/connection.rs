//! SQL Server connection management

use anyhow::{Context, Result};
use tiberius::{Client, Config, AuthMethod};
use tokio::net::TcpStream;
use tokio_util::compat::{TokioAsyncWriteCompatExt, Compat};
use std::sync::Arc;
use tokio::sync::Mutex;

/// Database configuration
#[derive(Clone, Debug)]
pub struct DbConfig {
    pub host: String,
    pub port: u16,
    pub user: String,
    pub password: String,
    pub database: String,
    pub encrypt: bool,
    pub trust_cert: bool,
}

impl Default for DbConfig {
    fn default() -> Self {
        Self {
            host: std::env::var("DB_HOST").unwrap_or_else(|_| "localhost".to_string()),
            port: std::env::var("DB_PORT").ok().and_then(|p| p.parse().ok()).unwrap_or(1433),
            user: std::env::var("DB_USER").unwrap_or_else(|_| "sa".to_string()),
            password: std::env::var("DB_PASSWORD").unwrap_or_else(|_| "".to_string()),
            database: std::env::var("DB_DATABASE").unwrap_or_else(|_| "master".to_string()),
            encrypt: false,
            trust_cert: true,
        }
    }
}

/// Database connection wrapper
pub struct DbConnection {
    client: Arc<Mutex<Client<Compat<TcpStream>>>>,
    pub config: DbConfig,
    pub connected: bool,
}

impl DbConnection {
    /// Create a new database connection
    pub async fn new(config: DbConfig) -> Result<Self> {
        let client = Self::connect(&config).await?;

        Ok(Self {
            client: Arc::new(Mutex::new(client)),
            config,
            connected: true,
        })
    }

    /// Connect to SQL Server
    async fn connect(db_config: &DbConfig) -> Result<Client<Compat<TcpStream>>> {
        let mut config = Config::new();

        config.host(&db_config.host);
        config.port(db_config.port);
        config.database(&db_config.database);
        config.authentication(AuthMethod::sql_server(&db_config.user, &db_config.password));

        if db_config.trust_cert {
            config.trust_cert();
        }

        if !db_config.encrypt {
            config.encryption(tiberius::EncryptionLevel::NotSupported);
        }

        println!("{}", config.get_addr());
        let tcp = TcpStream::connect(config.get_addr())
            .await
            .context("Failed to connect to SQL Server")?;

        tcp.set_nodelay(true)?;

        let client = Client::connect(config, tcp.compat_write())
            .await
            .context("Failed to authenticate with SQL Server")?;

        Ok(client)
    }

    /// Reconnect to the database
    pub async fn reconnect(&mut self) -> Result<()> {
        let client = Self::connect(&self.config).await?;
        *self.client.lock().await = client;
        self.connected = true;
        Ok(())
    }

    /// Get a reference to the client
    pub fn client(&self) -> Arc<Mutex<Client<Compat<TcpStream>>>> {
        Arc::clone(&self.client)
    }

    /// Test the connection
    pub async fn test_connection(&self) -> Result<bool> {
        let mut client = self.client.lock().await;
        let result = client.simple_query("SELECT 1").await;
        Ok(result.is_ok())
    }

    /// Get server version
    pub async fn get_server_version(&self) -> Result<String> {
        let mut client = self.client.lock().await;
        let stream = client.simple_query("SELECT @@VERSION").await?;
        let row = stream.into_row().await?.context("No version info")?;
        let version: &str = row.get(0).context("No version column")?;
        Ok(version.to_string())
    }
}
