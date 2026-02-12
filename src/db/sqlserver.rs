//! SQL Server driver implementation using tiberius
//!
//! Wraps the existing DbConnection / QueryExecutor / SchemaExplorer logic
//! behind the DatabaseDriver trait.

use crate::db::driver::{DatabaseBackend, DatabaseDriver};
use crate::db::query::{CellValue, ColumnInfo, QueryResult};
use crate::db::schema::{ColumnDef, DatabaseObject, ObjectType};
use anyhow::{Context, Result};
use async_trait::async_trait;
use std::sync::Arc;
use std::time::Instant;
use tiberius::time::chrono::{DateTime, FixedOffset, NaiveDate, NaiveDateTime, NaiveTime};
use tiberius::{Client, Column, ColumnType, Config, AuthMethod, Row, numeric::Numeric};
use tokio::net::TcpStream;
use tokio::sync::Mutex;
use tokio_util::compat::{Compat, TokioAsyncWriteCompatExt};

/// Configuration specific to SQL Server connections
#[derive(Clone, Debug)]
pub struct SqlServerConfig {
    pub host: String,
    pub port: u16,
    pub user: String,
    pub password: String,
    pub database: String,
    pub encrypt: bool,
    pub trust_cert: bool,
}

impl Default for SqlServerConfig {
    fn default() -> Self {
        Self {
            host: std::env::var("DB_HOST").unwrap_or_else(|_| "localhost".to_string()),
            port: std::env::var("DB_PORT").ok().and_then(|p| p.parse().ok()).unwrap_or(1433),
            user: std::env::var("DB_USER").unwrap_or_else(|_| "sa".to_string()),
            password: std::env::var("DB_PASSWORD").unwrap_or_else(|_| String::new()),
            database: std::env::var("DB_DATABASE").unwrap_or_else(|_| "master".to_string()),
            encrypt: false,
            trust_cert: true,
        }
    }
}

/// SQL Server driver
pub struct SqlServerDriver {
    client: Arc<Mutex<Client<Compat<TcpStream>>>>,
    pub config: SqlServerConfig,
}

impl SqlServerDriver {
    /// Create a new SQL Server connection
    pub async fn new(config: SqlServerConfig) -> Result<Self> {
        let client = Self::connect_internal(&config).await?;
        Ok(Self {
            client: Arc::new(Mutex::new(client)),
            config,
        })
    }

    /// Internal TCP + TDS connection
    async fn connect_internal(cfg: &SqlServerConfig) -> Result<Client<Compat<TcpStream>>> {
        let mut config = Config::new();
        config.host(&cfg.host);
        config.port(cfg.port);
        config.database(&cfg.database);
        config.authentication(AuthMethod::sql_server(&cfg.user, &cfg.password));

        if cfg.trust_cert {
            config.trust_cert();
        }
        if !cfg.encrypt {
            config.encryption(tiberius::EncryptionLevel::NotSupported);
        }

        let tcp = TcpStream::connect(config.get_addr())
            .await
            .context("Failed to connect to SQL Server")?;
        tcp.set_nodelay(true)?;

        let client = Client::connect(config, tcp.compat_write())
            .await
            .context("Failed to authenticate with SQL Server")?;

        Ok(client)
    }

    /// Get a cloneable reference to the underlying tiberius client.
    /// Needed for background tasks (column loading, query execution).
    pub fn client_arc(&self) -> Arc<Mutex<Client<Compat<TcpStream>>>> {
        Arc::clone(&self.client)
    }

    /// Execute a query using a raw client reference (for background tasks)
    pub async fn execute_query_with_client(
        client: &mut Client<Compat<TcpStream>>,
        query: &str,
    ) -> Result<QueryResult> {
        let start = Instant::now();
        let stream = client.simple_query(query).await?;
        Self::process_results(stream, start).await
    }

    // ---- helpers for query result processing ----

    async fn process_results(
        stream: tiberius::QueryStream<'_>,
        start: Instant,
    ) -> Result<QueryResult> {
        let mut columns: Vec<ColumnInfo> = Vec::new();
        let mut rows: Vec<Vec<CellValue>> = Vec::new();

        let results = stream.into_results().await?;
        for result in results {
            for row in result {
                if columns.is_empty() {
                    columns = row
                        .columns()
                        .iter()
                        .map(|c| ColumnInfo {
                            name: c.name().to_string(),
                            type_name: format_column_type(c),
                            max_width: c.name().len().max(4),
                        })
                        .collect();
                }

                let mut row_data: Vec<CellValue> = Vec::new();
                for (i, col) in row.columns().iter().enumerate() {
                    let value = extract_cell_value(&row, i, col);
                    let value_len = value.to_string().len();
                    if i < columns.len() {
                        columns[i].max_width = columns[i].max_width.max(value_len);
                    }
                    row_data.push(value);
                }
                rows.push(row_data);
            }
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

    /// Helper: run a query and collect string column 0 from all rows
    async fn collect_strings(&self, query: &str) -> Result<Vec<String>> {
        let mut client = self.client.lock().await;
        let stream = client.simple_query(query).await?;
        let results = stream.into_results().await?;
        let mut out = Vec::new();
        for result in results {
            for row in result {
                if let Some(name) = row.get::<&str, _>(0) {
                    out.push(name.to_string());
                }
            }
        }
        Ok(out)
    }

    /// Helper: run a query and collect DatabaseObjects (schema col 0, name col 1)
    async fn collect_objects(&self, query: &str, obj_type: ObjectType) -> Result<Vec<DatabaseObject>> {
        let mut client = self.client.lock().await;
        let stream = client.simple_query(query).await?;
        let results = stream.into_results().await?;
        let mut out = Vec::new();
        for result in results {
            for row in result {
                let schema = row.get::<&str, _>(0).unwrap_or("dbo").to_string();
                let name = row.get::<&str, _>(1).unwrap_or("").to_string();
                out.push(DatabaseObject { name, schema, object_type: obj_type.clone() });
            }
        }
        Ok(out)
    }
}

#[async_trait]
impl DatabaseDriver for SqlServerDriver {
    fn backend(&self) -> DatabaseBackend {
        DatabaseBackend::SqlServer
    }

    async fn test_connection(&self) -> Result<bool> {
        let mut client = self.client.lock().await;
        Ok(client.simple_query("SELECT 1").await.is_ok())
    }

    async fn get_server_version(&self) -> Result<String> {
        let mut client = self.client.lock().await;
        let stream = client.simple_query("SELECT @@VERSION").await?;
        let row = stream.into_row().await?.context("No version info")?;
        let version: &str = row.get(0).context("No version column")?;
        Ok(version.to_string())
    }

    async fn reconnect(&mut self) -> Result<()> {
        let client = Self::connect_internal(&self.config).await?;
        *self.client.lock().await = client;
        Ok(())
    }

    async fn execute_query(&self, query: &str) -> Result<QueryResult> {
        let start = Instant::now();
        let mut client = self.client.lock().await;
        let stream = client.simple_query(query).await?;
        Self::process_results(stream, start).await
    }

    fn database_name(&self) -> String {
        self.config.database.clone()
    }

    async fn get_databases(&self) -> Result<Vec<String>> {
        self.collect_strings("SELECT name FROM sys.databases WHERE state = 0 ORDER BY name").await
    }

    async fn get_schemas(&self) -> Result<Vec<String>> {
        self.collect_strings("SELECT name FROM sys.schemas WHERE schema_id < 16384 ORDER BY name").await
    }

    async fn get_tables(&self, schema_filter: Option<&str>) -> Result<Vec<DatabaseObject>> {
        let query = match schema_filter {
            Some(schema) => format!(
                "SELECT s.name, t.name FROM sys.tables t \
                 INNER JOIN sys.schemas s ON t.schema_id = s.schema_id \
                 WHERE s.name = '{}' ORDER BY s.name, t.name",
                schema
            ),
            None => "SELECT s.name, t.name FROM sys.tables t \
                     INNER JOIN sys.schemas s ON t.schema_id = s.schema_id \
                     ORDER BY s.name, t.name"
                .to_string(),
        };
        self.collect_objects(&query, ObjectType::Table).await
    }

    async fn get_views(&self, schema_filter: Option<&str>) -> Result<Vec<DatabaseObject>> {
        let query = match schema_filter {
            Some(schema) => format!(
                "SELECT s.name, v.name FROM sys.views v \
                 INNER JOIN sys.schemas s ON v.schema_id = s.schema_id \
                 WHERE s.name = '{}' ORDER BY s.name, v.name",
                schema
            ),
            None => "SELECT s.name, v.name FROM sys.views v \
                     INNER JOIN sys.schemas s ON v.schema_id = s.schema_id \
                     ORDER BY s.name, v.name"
                .to_string(),
        };
        self.collect_objects(&query, ObjectType::View).await
    }

    async fn get_columns(&self, schema: &str, table: &str) -> Result<Vec<ColumnDef>> {
        let query = format!(
            "SELECT c.name, t.name, c.is_nullable, \
             ISNULL(pk.is_primary_key, 0), c.is_identity, \
             c.max_length, c.precision, c.scale \
             FROM sys.columns c \
             INNER JOIN sys.types t ON c.user_type_id = t.user_type_id \
             INNER JOIN sys.tables tbl ON c.object_id = tbl.object_id \
             INNER JOIN sys.schemas s ON tbl.schema_id = s.schema_id \
             LEFT JOIN ( \
                SELECT ic.column_id, ic.object_id, 1 as is_primary_key \
                FROM sys.index_columns ic \
                INNER JOIN sys.indexes i ON ic.object_id = i.object_id AND ic.index_id = i.index_id \
                WHERE i.is_primary_key = 1 \
             ) pk ON c.object_id = pk.object_id AND c.column_id = pk.column_id \
             WHERE s.name = '{}' AND tbl.name = '{}' \
             ORDER BY c.column_id",
            schema, table
        );

        let mut client = self.client.lock().await;
        let stream = client.simple_query(&query).await?;
        let results = stream.into_results().await?;

        let mut columns = Vec::new();
        for result in results {
            for row in result {
                columns.push(ColumnDef {
                    name: row.get::<&str, _>(0).unwrap_or("").to_string(),
                    data_type: row.get::<&str, _>(1).unwrap_or("").to_string(),
                    is_nullable: row.get::<bool, _>(2).unwrap_or(true),
                    is_primary_key: row.get::<i32, _>(3).unwrap_or(0) == 1,
                    is_identity: row.get::<bool, _>(4).unwrap_or(false),
                    max_length: row.get::<i16, _>(5).map(|v| v as i32),
                    precision: row.get::<u8, _>(6).map(|v| v as i32),
                    scale: row.get::<u8, _>(7).map(|v| v as i32),
                });
            }
        }
        Ok(columns)
    }

    async fn get_procedures(&self, schema_filter: Option<&str>) -> Result<Vec<DatabaseObject>> {
        let query = match schema_filter {
            Some(schema) => format!(
                "SELECT s.name, p.name FROM sys.procedures p \
                 INNER JOIN sys.schemas s ON p.schema_id = s.schema_id \
                 WHERE s.name = '{}' ORDER BY s.name, p.name",
                schema
            ),
            None => "SELECT s.name, p.name FROM sys.procedures p \
                     INNER JOIN sys.schemas s ON p.schema_id = s.schema_id \
                     ORDER BY s.name, p.name"
                .to_string(),
        };
        self.collect_objects(&query, ObjectType::StoredProcedure).await
    }

    async fn get_procedure_definition(&self, schema: &str, name: &str) -> Result<String> {
        let query = format!(
            "SELECT OBJECT_NAME(object_id), definition \
             FROM sys.sql_modules \
             WHERE OBJECT_SCHEMA_NAME(object_id) = '{}' AND OBJECT_NAME(object_id) = '{}'",
            schema, name
        );

        let mut client = self.client.lock().await;
        let stream = client.simple_query(&query).await?;
        let row = stream.into_row().await?.context("No procedure definition")?;
        let definition = row.get::<&str, _>(1).unwrap_or("");

        Ok(definition
            .replace("\r\n", "\n")
            .replace('\r', "\n")
            .replace('\t', "    ")
            .replace("CREATE PROCEDURE", "ALTER PROCEDURE"))
    }

    async fn get_table_row_count(&self, schema: &str, table: &str) -> Result<i64> {
        let query = format!(
            "SELECT SUM(p.rows) FROM sys.partitions p \
             INNER JOIN sys.tables t ON p.object_id = t.object_id \
             INNER JOIN sys.schemas s ON t.schema_id = s.schema_id \
             WHERE s.name = '{}' AND t.name = '{}' AND p.index_id IN (0, 1)",
            schema, table
        );

        let mut client = self.client.lock().await;
        let stream = client.simple_query(&query).await?;
        let row = stream.into_row().await?.context("No row count")?;
        Ok(row.get::<i64, _>(0).unwrap_or(0))
    }

    async fn get_table_ddl(&self, schema: &str, table: &str) -> Result<String> {
        let columns = self.get_columns(schema, table).await?;

        let mut ddl = format!("CREATE TABLE [{}].[{}] (\n", schema, table);
        for (i, col) in columns.iter().enumerate() {
            let type_str = if col.data_type == "varchar" || col.data_type == "nvarchar" {
                if col.max_length == Some(-1) {
                    format!("{}(MAX)", col.data_type.to_uppercase())
                } else {
                    format!("{}({})", col.data_type.to_uppercase(), col.max_length.unwrap_or(0))
                }
            } else if col.data_type == "decimal" || col.data_type == "numeric" {
                format!(
                    "{}({}, {})",
                    col.data_type.to_uppercase(),
                    col.precision.unwrap_or(18),
                    col.scale.unwrap_or(0)
                )
            } else {
                col.data_type.to_uppercase()
            };

            let nullable = if col.is_nullable { "NULL" } else { "NOT NULL" };
            let pk = if col.is_primary_key { " PRIMARY KEY" } else { "" };
            let comma = if i < columns.len() - 1 { "," } else { "" };
            ddl.push_str(&format!("    [{}] {} {}{}{}\n", col.name, type_str, nullable, pk, comma));
        }
        ddl.push_str(");");
        Ok(ddl)
    }

    async fn search_objects(&self, search_term: &str) -> Result<Vec<DatabaseObject>> {
        let query = format!(
            "SELECT s.name, o.name, o.type_desc \
             FROM sys.objects o \
             INNER JOIN sys.schemas s ON o.schema_id = s.schema_id \
             WHERE o.name LIKE '%{}%' AND o.type IN ('U', 'V', 'P', 'FN', 'IF', 'TF') \
             ORDER BY o.type, s.name, o.name",
            search_term
        );

        let mut client = self.client.lock().await;
        let stream = client.simple_query(&query).await?;
        let results = stream.into_results().await?;

        let mut objects = Vec::new();
        for result in results {
            for row in result {
                let schema = row.get::<&str, _>(0).unwrap_or("dbo").to_string();
                let name = row.get::<&str, _>(1).unwrap_or("").to_string();
                let type_desc = row.get::<&str, _>(2).unwrap_or("");

                let object_type = match type_desc {
                    "USER_TABLE" => ObjectType::Table,
                    "VIEW" => ObjectType::View,
                    "SQL_STORED_PROCEDURE" => ObjectType::StoredProcedure,
                    _ => ObjectType::Function,
                };
                objects.push(DatabaseObject { name, schema, object_type });
            }
        }
        Ok(objects)
    }
}

// ---- Type conversion helpers (moved from query.rs) ----

fn format_column_type(col: &Column) -> String {
    match col.column_type() {
        ColumnType::Null => "NULL".to_string(),
        ColumnType::Bit | ColumnType::Bitn => "BIT".to_string(),
        ColumnType::Int1 => "TINYINT".to_string(),
        ColumnType::Int2 => "SMALLINT".to_string(),
        ColumnType::Int4 => "INT".to_string(),
        ColumnType::Int8 => "BIGINT".to_string(),
        ColumnType::Float4 => "REAL".to_string(),
        ColumnType::Float8 => "FLOAT".to_string(),
        ColumnType::Datetime | ColumnType::Datetimen => "DATETIME".to_string(),
        ColumnType::Datetime2 => "DATETIME2".to_string(),
        ColumnType::DatetimeOffsetn => "DATETIMEOFFSET".to_string(),
        ColumnType::Daten => "DATE".to_string(),
        ColumnType::Timen => "TIME".to_string(),
        ColumnType::Decimaln => "DECIMAL".to_string(),
        ColumnType::Numericn => "NUMERIC".to_string(),
        ColumnType::Money => "MONEY".to_string(),
        ColumnType::Money4 => "SMALLMONEY".to_string(),
        ColumnType::Guid => "UNIQUEIDENTIFIER".to_string(),
        ColumnType::BigVarChar => "VARCHAR(MAX)".to_string(),
        ColumnType::BigChar => "CHAR".to_string(),
        ColumnType::NVarchar => "NVARCHAR".to_string(),
        ColumnType::NChar => "NCHAR".to_string(),
        ColumnType::Text => "TEXT".to_string(),
        ColumnType::NText => "NTEXT".to_string(),
        ColumnType::BigVarBin => "VARBINARY(MAX)".to_string(),
        ColumnType::BigBinary => "BINARY".to_string(),
        ColumnType::Image => "IMAGE".to_string(),
        ColumnType::Xml => "XML".to_string(),
        _ => "UNKNOWN".to_string(),
    }
}

fn extract_cell_value(row: &Row, index: usize, col: &Column) -> CellValue {
    match col.column_type() {
        ColumnType::Null => CellValue::Null,
        ColumnType::Bit | ColumnType::Bitn => row
            .get::<bool, _>(index)
            .map(CellValue::Bool)
            .unwrap_or(CellValue::Null),
        ColumnType::Int1 => row
            .get::<u8, _>(index)
            .map(|v| CellValue::Int(v as i64))
            .unwrap_or(CellValue::Null),
        ColumnType::Int2 => row
            .get::<i16, _>(index)
            .map(|v| CellValue::Int(v as i64))
            .unwrap_or(CellValue::Null),
        ColumnType::Int4 => row
            .get::<i32, _>(index)
            .map(|v| CellValue::Int(v as i64))
            .unwrap_or(CellValue::Null),
        ColumnType::Int8 => row
            .get::<i64, _>(index)
            .map(CellValue::Int)
            .unwrap_or(CellValue::Null),
        ColumnType::Float4 => row
            .get::<f32, _>(index)
            .map(|v| CellValue::Float(v as f64))
            .unwrap_or(CellValue::Null),
        ColumnType::Float8 => row
            .get::<f64, _>(index)
            .map(CellValue::Float)
            .unwrap_or(CellValue::Null),
        ColumnType::Decimaln | ColumnType::Numericn => row
            .get::<Numeric, _>(index)
            .map(|v| CellValue::String(v.to_string()))
            .unwrap_or(CellValue::Null),
        ColumnType::Money | ColumnType::Money4 => row
            .get::<f64, _>(index)
            .map(CellValue::Float)
            .unwrap_or(CellValue::Null),
        ColumnType::Datetime | ColumnType::Datetime2 | ColumnType::Datetimen => row
            .get::<NaiveDateTime, _>(index)
            .map(|v| CellValue::DateTime(v.format("%Y-%m-%d %H:%M:%S").to_string()))
            .unwrap_or(CellValue::Null),
        ColumnType::Daten => row
            .get::<NaiveDate, _>(index)
            .map(|v| CellValue::DateTime(v.format("%Y-%m-%d").to_string()))
            .unwrap_or(CellValue::Null),
        ColumnType::Timen => row
            .get::<NaiveTime, _>(index)
            .map(|v| CellValue::DateTime(v.format("%H:%M:%S").to_string()))
            .unwrap_or(CellValue::Null),
        ColumnType::DatetimeOffsetn => row
            .get::<DateTime<FixedOffset>, _>(index)
            .map(|v| CellValue::DateTime(v.format("%Y-%m-%d %H:%M:%S %:z").to_string()))
            .unwrap_or(CellValue::Null),
        ColumnType::BigVarChar
        | ColumnType::BigChar
        | ColumnType::NVarchar
        | ColumnType::NChar
        | ColumnType::Text
        | ColumnType::NText
        | ColumnType::Xml => row
            .get::<&str, _>(index)
            .map(|v| CellValue::String(v.to_string()))
            .unwrap_or(CellValue::Null),
        ColumnType::Guid => row
            .get::<tiberius::Uuid, _>(index)
            .map(|v| CellValue::String(v.to_string()))
            .unwrap_or(CellValue::Null),
        ColumnType::BigVarBin | ColumnType::BigBinary | ColumnType::Image => row
            .get::<&[u8], _>(index)
            .map(|v| CellValue::Binary(v.to_vec()))
            .unwrap_or(CellValue::Null),
        _ => {
            if let Some(v) = row.try_get::<&str, _>(index).ok().flatten() {
                return CellValue::String(v.to_string());
            }
            if let Some(v) = row.try_get::<NaiveDateTime, _>(index).ok().flatten() {
                return CellValue::DateTime(v.format("%Y-%m-%d %H:%M:%S").to_string());
            }
            if let Some(v) = row.try_get::<i64, _>(index).ok().flatten() {
                return CellValue::Int(v);
            }
            if let Some(v) = row.try_get::<f64, _>(index).ok().flatten() {
                return CellValue::Float(v);
            }
            if let Some(v) = row.try_get::<Numeric, _>(index).ok().flatten() {
                return CellValue::String(v.to_string());
            }
            CellValue::String(format!("<{:?}>", col.column_type()))
        }
    }
}
