//! SQLite driver implementation using rusqlite
//!
//! Uses `spawn_blocking` to bridge rusqlite's synchronous API
//! into the async world expected by DatabaseDriver.

use crate::db::driver::{DatabaseBackend, DatabaseDriver};
use crate::db::query::{CellValue, ColumnInfo, QueryResult};
use crate::db::schema::{ColumnDef, DatabaseObject, ObjectType};
use anyhow::{Context, Result};
use async_trait::async_trait;
use rusqlite::{Connection, types::ValueRef};
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Instant;
use tokio::sync::Mutex;

/// SQLite driver
pub struct SqliteDriver {
    conn: Arc<Mutex<Connection>>,
    pub path: PathBuf,
}

impl SqliteDriver {
    /// Open (or create) a SQLite database file
    pub async fn new(path: PathBuf) -> Result<Self> {
        let p = path.clone();
        let conn = tokio::task::spawn_blocking(move || {
            Connection::open(&p).context("Failed to open SQLite database")
        })
        .await??;

        Ok(Self {
            conn: Arc::new(Mutex::new(conn)),
            path,
        })
    }
}

#[async_trait]
impl DatabaseDriver for SqliteDriver {
    fn backend(&self) -> DatabaseBackend {
        DatabaseBackend::Sqlite
    }

    async fn test_connection(&self) -> Result<bool> {
        let conn = self.conn.lock().await;
        // rusqlite is sync but we're already holding the lock
        // For a quick check this is fine
        Ok(conn.execute_batch("SELECT 1").is_ok())
    }

    async fn get_server_version(&self) -> Result<String> {
        let conn = self.conn.lock().await;
        let version: String = conn.query_row("SELECT sqlite_version()", [], |row| row.get(0))?;
        Ok(format!("SQLite {}", version))
    }

    async fn reconnect(&mut self) -> Result<()> {
        let p = self.path.clone();
        let conn = tokio::task::spawn_blocking(move || {
            Connection::open(&p).context("Failed to reopen SQLite database")
        })
        .await??;
        *self.conn.lock().await = conn;
        Ok(())
    }

    async fn execute_query(&self, query: &str) -> Result<QueryResult> {
        let conn = self.conn.lock().await;
        let start = Instant::now();

        // Try as a query that returns rows first
        let mut stmt = conn.prepare(query)?;
        let col_count = stmt.column_count();

        if col_count == 0 {
            // Statement doesn't return rows (INSERT/UPDATE/DELETE/CREATE/etc.)
            drop(stmt);
            let affected = conn.execute_batch(query);
            return Ok(QueryResult {
                columns: Vec::new(),
                rows: Vec::new(),
                row_count: 0,
                execution_time: start.elapsed(),
                affected_rows: affected.ok().map(|_| 0),
                messages: Vec::new(),
            });
        }

        // Build column info
        let mut columns: Vec<ColumnInfo> = (0..col_count)
            .map(|i| {
                let name = stmt.column_name(i).unwrap_or("?").to_string();
                let max_w = name.len().max(4);
                ColumnInfo {
                    name,
                    type_name: "TEXT".to_string(), // will be refined per-row
                    max_width: max_w,
                }
            })
            .collect();

        let mut rows: Vec<Vec<CellValue>> = Vec::new();
        let mut raw_rows = stmt.query([])?;

        while let Some(row) = raw_rows.next()? {
            let mut row_data = Vec::with_capacity(col_count);
            for i in 0..col_count {
                let val = match row.get_ref(i)? {
                    ValueRef::Null => CellValue::Null,
                    ValueRef::Integer(v) => CellValue::Int(v),
                    ValueRef::Real(v) => CellValue::Float(v),
                    ValueRef::Text(v) => {
                        let s = String::from_utf8_lossy(v).to_string();
                        CellValue::String(s)
                    }
                    ValueRef::Blob(v) => CellValue::Binary(v.to_vec()),
                };
                let val_len = val.to_string().len();
                if i < columns.len() {
                    columns[i].max_width = columns[i].max_width.max(val_len);
                }
                // Update type_name based on first non-null value
                if rows.is_empty() {
                    columns[i].type_name = match &val {
                        CellValue::Null => "NULL".to_string(),
                        CellValue::Int(_) => "INTEGER".to_string(),
                        CellValue::Float(_) => "REAL".to_string(),
                        CellValue::String(_) => "TEXT".to_string(),
                        CellValue::Binary(_) => "BLOB".to_string(),
                        _ => "TEXT".to_string(),
                    };
                }
                row_data.push(val);
            }
            rows.push(row_data);
        }

        Ok(QueryResult {
            row_count: rows.len(),
            columns,
            rows,
            execution_time: start.elapsed(),
            affected_rows: None,
            messages: Vec::new(),
        })
    }

    fn database_name(&self) -> String {
        self.path
            .file_name()
            .map(|f| f.to_string_lossy().to_string())
            .unwrap_or_else(|| "sqlite".to_string())
    }

    async fn get_databases(&self) -> Result<Vec<String>> {
        // SQLite: list attached databases
        let conn = self.conn.lock().await;
        let mut stmt = conn.prepare("PRAGMA database_list")?;
        let mut dbs = Vec::new();
        let mut rows = stmt.query([])?;
        while let Some(row) = rows.next()? {
            let name: String = row.get(1)?;
            dbs.push(name);
        }
        Ok(dbs)
    }

    async fn get_schemas(&self) -> Result<Vec<String>> {
        Ok(vec!["main".to_string()])
    }

    async fn get_tables(&self, _schema_filter: Option<&str>) -> Result<Vec<DatabaseObject>> {
        let conn = self.conn.lock().await;
        let mut stmt = conn.prepare(
            "SELECT name FROM sqlite_master WHERE type='table' AND name NOT LIKE 'sqlite_%' ORDER BY name",
        )?;
        let mut tables = Vec::new();
        let mut rows = stmt.query([])?;
        while let Some(row) = rows.next()? {
            let name: String = row.get(0)?;
            tables.push(DatabaseObject {
                name,
                schema: "main".to_string(),
                object_type: ObjectType::Table,
            });
        }
        Ok(tables)
    }

    async fn get_views(&self, _schema_filter: Option<&str>) -> Result<Vec<DatabaseObject>> {
        let conn = self.conn.lock().await;
        let mut stmt = conn.prepare(
            "SELECT name FROM sqlite_master WHERE type='view' ORDER BY name",
        )?;
        let mut views = Vec::new();
        let mut rows = stmt.query([])?;
        while let Some(row) = rows.next()? {
            let name: String = row.get(0)?;
            views.push(DatabaseObject {
                name,
                schema: "main".to_string(),
                object_type: ObjectType::View,
            });
        }
        Ok(views)
    }

    async fn get_columns(&self, _schema: &str, table: &str) -> Result<Vec<ColumnDef>> {
        let conn = self.conn.lock().await;
        let query = format!("PRAGMA table_info('{}')", table);
        let mut stmt = conn.prepare(&query)?;
        let mut columns = Vec::new();
        let mut rows = stmt.query([])?;
        while let Some(row) = rows.next()? {
            let name: String = row.get(1)?;
            let data_type: String = row.get(2)?;
            let not_null: bool = row.get(3)?;
            let pk: i32 = row.get(5)?;

            columns.push(ColumnDef {
                name,
                data_type,
                is_nullable: !not_null,
                is_primary_key: pk > 0,
                is_identity: false, // SQLite AUTOINCREMENT is implicit via INTEGER PRIMARY KEY
                max_length: None,
                precision: None,
                scale: None,
            });
        }
        Ok(columns)
    }

    async fn get_procedures(&self, _schema_filter: Option<&str>) -> Result<Vec<DatabaseObject>> {
        // SQLite doesn't have stored procedures
        Ok(Vec::new())
    }

    async fn get_procedure_definition(&self, _schema: &str, _name: &str) -> Result<String> {
        anyhow::bail!("SQLite does not support stored procedures")
    }

    async fn get_table_row_count(&self, _schema: &str, table: &str) -> Result<i64> {
        let conn = self.conn.lock().await;
        let query = format!("SELECT COUNT(*) FROM \"{}\"", table);
        let count: i64 = conn.query_row(&query, [], |row| row.get(0))?;
        Ok(count)
    }

    async fn get_table_ddl(&self, _schema: &str, table: &str) -> Result<String> {
        let conn = self.conn.lock().await;
        let sql: String = conn.query_row(
            "SELECT sql FROM sqlite_master WHERE type='table' AND name=?1",
            [table],
            |row| row.get(0),
        )?;
        Ok(sql)
    }

    async fn search_objects(&self, search_term: &str) -> Result<Vec<DatabaseObject>> {
        let conn = self.conn.lock().await;
        let query = format!(
            "SELECT name, type FROM sqlite_master WHERE name LIKE '%{}%' AND type IN ('table', 'view') ORDER BY type, name",
            search_term
        );
        let mut stmt = conn.prepare(&query)?;
        let mut objects = Vec::new();
        let mut rows = stmt.query([])?;
        while let Some(row) = rows.next()? {
            let name: String = row.get(0)?;
            let obj_type: String = row.get(1)?;
            objects.push(DatabaseObject {
                name,
                schema: "main".to_string(),
                object_type: if obj_type == "table" {
                    ObjectType::Table
                } else {
                    ObjectType::View
                },
            });
        }
        Ok(objects)
    }
}
