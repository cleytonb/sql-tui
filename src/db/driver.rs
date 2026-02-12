//! Database driver abstraction trait
//!
//! Defines the interface that all database backends must implement.

use crate::db::{ColumnDef, DatabaseObject, QueryResult};
use anyhow::Result;
use async_trait::async_trait;

/// Which database backend is in use
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum DatabaseBackend {
    SqlServer,
    Sqlite,
}

impl std::fmt::Display for DatabaseBackend {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DatabaseBackend::SqlServer => write!(f, "SQL Server"),
            DatabaseBackend::Sqlite => write!(f, "SQLite"),
        }
    }
}

impl Default for DatabaseBackend {
    fn default() -> Self {
        DatabaseBackend::SqlServer
    }
}

/// Trait that all database drivers must implement.
///
/// All methods are async because the caller (App) lives in a tokio runtime.
/// Synchronous drivers (like rusqlite) should use `spawn_blocking` internally.
#[async_trait]
pub trait DatabaseDriver: Send + Sync {
    /// Which backend this driver represents
    fn backend(&self) -> DatabaseBackend;

    /// Test that the connection is alive
    async fn test_connection(&self) -> Result<bool>;

    /// Get a human-readable server/engine version string
    async fn get_server_version(&self) -> Result<String>;

    /// Reconnect using the same configuration
    async fn reconnect(&mut self) -> Result<()>;

    /// Execute a SQL query and return results
    async fn execute_query(&self, query: &str) -> Result<QueryResult>;

    /// Get the name of the current database / file
    fn database_name(&self) -> String;

    // --- Schema exploration ---

    /// List available databases (SQL Server) or attached databases (SQLite)
    async fn get_databases(&self) -> Result<Vec<String>>;

    /// List schemas in the current database.
    /// SQLite returns just `["main"]`.
    async fn get_schemas(&self) -> Result<Vec<String>>;

    /// List tables, optionally filtered by schema
    async fn get_tables(&self, schema_filter: Option<&str>) -> Result<Vec<DatabaseObject>>;

    /// List views, optionally filtered by schema
    async fn get_views(&self, schema_filter: Option<&str>) -> Result<Vec<DatabaseObject>>;

    /// Get column definitions for a table or view
    async fn get_columns(&self, schema: &str, table: &str) -> Result<Vec<ColumnDef>>;

    /// List stored procedures (returns empty vec for SQLite)
    async fn get_procedures(&self, schema_filter: Option<&str>) -> Result<Vec<DatabaseObject>>;

    /// Get stored procedure definition (returns error for SQLite)
    async fn get_procedure_definition(&self, schema: &str, name: &str) -> Result<String>;

    /// Estimate row count for a table
    async fn get_table_row_count(&self, schema: &str, table: &str) -> Result<i64>;

    /// Generate CREATE TABLE DDL for a table
    async fn get_table_ddl(&self, schema: &str, table: &str) -> Result<String>;

    /// Search for objects by name
    async fn search_objects(&self, search_term: &str) -> Result<Vec<DatabaseObject>>;
}
