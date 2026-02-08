//! Schema exploration and metadata

use anyhow::{Context, Result};
use tiberius::Client;
use tokio::net::TcpStream;
use tokio_util::compat::Compat;

/// Database object
#[derive(Clone, Debug)]
pub struct DatabaseObject {
    pub name: String,
    pub object_type: ObjectType,
    pub schema: String,
}

/// Object type
#[derive(Clone, Debug, PartialEq)]
pub enum ObjectType {
    Database,
    Schema,
    Table,
    View,
    StoredProcedure,
    Function,
    Column,
    Index,
}

impl std::fmt::Display for ObjectType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ObjectType::Database => write!(f, "Database"),
            ObjectType::Schema => write!(f, "Schema"),
            ObjectType::Table => write!(f, "Table"),
            ObjectType::View => write!(f, "View"),
            ObjectType::StoredProcedure => write!(f, "Procedure"),
            ObjectType::Function => write!(f, "Function"),
            ObjectType::Column => write!(f, "Column"),
            ObjectType::Index => write!(f, "Index"),
        }
    }
}

/// Column definition
#[derive(Clone, Debug)]
pub struct ColumnDef {
    pub name: String,
    pub data_type: String,
    pub is_nullable: bool,
    pub is_primary_key: bool,
    pub is_identity: bool,
    pub max_length: Option<i32>,
    pub precision: Option<i32>,
    pub scale: Option<i32>,
}

/// Table definition
#[derive(Clone, Debug)]
pub struct TableDef {
    pub schema: String,
    pub name: String,
    pub columns: Vec<ColumnDef>,
    pub row_count: Option<i64>,
}

/// Schema explorer
pub struct SchemaExplorer;

impl SchemaExplorer {
    /// Get all databases
    pub async fn get_databases(
        client: &mut Client<Compat<TcpStream>>,
    ) -> Result<Vec<String>> {
        let query = "SELECT name FROM sys.databases WHERE state = 0 ORDER BY name";
        let stream = client.simple_query(query).await?;
        let results = stream.into_results().await?;

        let mut databases = Vec::new();
        for result in results {
            for row in result {
                if let Some(name) = row.get::<&str, _>(0) {
                    databases.push(name.to_string());
                }
            }
        }

        Ok(databases)
    }

    /// Get all schemas in current database
    pub async fn get_schemas(
        client: &mut Client<Compat<TcpStream>>,
    ) -> Result<Vec<String>> {
        let query = "SELECT name FROM sys.schemas WHERE schema_id < 16384 ORDER BY name";
        let stream = client.simple_query(query).await?;
        let results = stream.into_results().await?;

        let mut schemas = Vec::new();
        for result in results {
            for row in result {
                if let Some(name) = row.get::<&str, _>(0) {
                    schemas.push(name.to_string());
                }
            }
        }

        Ok(schemas)
    }

    /// Get all tables in current database
    pub async fn get_tables(
        client: &mut Client<Compat<TcpStream>>,
        schema_filter: Option<&str>,
    ) -> Result<Vec<DatabaseObject>> {
        let query = match schema_filter {
            Some(schema) => format!(
                "SELECT s.name as schema_name, t.name as table_name
                 FROM sys.tables t
                 INNER JOIN sys.schemas s ON t.schema_id = s.schema_id
                 WHERE s.name = '{}'
                 ORDER BY s.name, t.name",
                schema
            ),
            None => "SELECT s.name as schema_name, t.name as table_name
                     FROM sys.tables t
                     INNER JOIN sys.schemas s ON t.schema_id = s.schema_id
                     ORDER BY s.name, t.name".to_string(),
        };

        let stream = client.simple_query(&query).await?;
        let results = stream.into_results().await?;

        let mut tables = Vec::new();
        for result in results {
            for row in result {
                let schema = row.get::<&str, _>(0).unwrap_or("dbo").to_string();
                let name = row.get::<&str, _>(1).unwrap_or("").to_string();
                tables.push(DatabaseObject {
                    name,
                    schema,
                    object_type: ObjectType::Table,
                });
            }
        }

        Ok(tables)
    }

    /// Get all views in current database
    pub async fn get_views(
        client: &mut Client<Compat<TcpStream>>,
        schema_filter: Option<&str>,
    ) -> Result<Vec<DatabaseObject>> {
        let query = match schema_filter {
            Some(schema) => format!(
                "SELECT s.name as schema_name, v.name as view_name
                 FROM sys.views v
                 INNER JOIN sys.schemas s ON v.schema_id = s.schema_id
                 WHERE s.name = '{}'
                 ORDER BY s.name, v.name",
                schema
            ),
            None => "SELECT s.name as schema_name, v.name as view_name
                     FROM sys.views v
                     INNER JOIN sys.schemas s ON v.schema_id = s.schema_id
                     ORDER BY s.name, v.name".to_string(),
        };

        let stream = client.simple_query(&query).await?;
        let results = stream.into_results().await?;

        let mut views = Vec::new();
        for result in results {
            for row in result {
                let schema = row.get::<&str, _>(0).unwrap_or("dbo").to_string();
                let name = row.get::<&str, _>(1).unwrap_or("").to_string();
                views.push(DatabaseObject {
                    name,
                    schema,
                    object_type: ObjectType::View,
                });
            }
        }

        Ok(views)
    }

    /// Get columns for a table
    pub async fn get_columns(
        client: &mut Client<Compat<TcpStream>>,
        schema: &str,
        table: &str,
    ) -> Result<Vec<ColumnDef>> {
        let query = format!(
            "SELECT
                c.name as column_name,
                t.name as data_type,
                c.is_nullable,
                ISNULL(pk.is_primary_key, 0) as is_primary_key,
                c.is_identity,
                c.max_length,
                c.precision,
                c.scale
             FROM sys.columns c
             INNER JOIN sys.types t ON c.user_type_id = t.user_type_id
             INNER JOIN sys.tables tbl ON c.object_id = tbl.object_id
             INNER JOIN sys.schemas s ON tbl.schema_id = s.schema_id
             LEFT JOIN (
                SELECT ic.column_id, ic.object_id, 1 as is_primary_key
                FROM sys.index_columns ic
                INNER JOIN sys.indexes i ON ic.object_id = i.object_id AND ic.index_id = i.index_id
                WHERE i.is_primary_key = 1
             ) pk ON c.object_id = pk.object_id AND c.column_id = pk.column_id
             WHERE s.name = '{}' AND tbl.name = '{}'
             ORDER BY c.column_id",
            schema, table
        );

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

    /// Get stored procedures
    pub async fn get_procedures(
        client: &mut Client<Compat<TcpStream>>,
        schema_filter: Option<&str>,
    ) -> Result<Vec<DatabaseObject>> {
        let query = match schema_filter {
            Some(schema) => format!(
                "SELECT s.name as schema_name, p.name as proc_name
                 FROM sys.procedures p
                 INNER JOIN sys.schemas s ON p.schema_id = s.schema_id
                 WHERE s.name = '{}'
                 ORDER BY s.name, p.name",
                schema
            ),
            None => "SELECT s.name as schema_name, p.name as proc_name
                     FROM sys.procedures p
                     INNER JOIN sys.schemas s ON p.schema_id = s.schema_id
                     ORDER BY s.name, p.name".to_string(),
        };

        let stream = client.simple_query(&query).await?;
        let results = stream.into_results().await?;

        let mut procs = Vec::new();
        for result in results {
            for row in result {
                let schema = row.get::<&str, _>(0).unwrap_or("dbo").to_string();
                let name = row.get::<&str, _>(1).unwrap_or("").to_string();
                procs.push(DatabaseObject {
                    name,
                    schema,
                    object_type: ObjectType::StoredProcedure,
                });
            }
        }

        Ok(procs)
    }

    /// Get stored procedures
    pub async fn get_procedure_definition(
        client: &mut Client<Compat<TcpStream>>,
        schema: String,
        name: String,
    ) -> Result<String> {
        let query = format!(
            "SELECT
                OBJECT_NAME(object_id) AS ProcedureName,
                definition AS ProcedureText
            FROM sys.sql_modules
            WHERE OBJECT_SCHEMA_NAME(object_id) = '{}' AND OBJECT_NAME(object_id) = '{}'",
            schema, name
        );

        let stream = client.simple_query(&query).await?;
        let row = stream.into_row().await?.context("No row count")?;
        let definition = row.get::<&str, _>(1).unwrap_or("");

        // Normalize line endings (CRLF -> LF) and tabs -> spaces
        let normalized = definition
            .replace("\r\n", "\n")
            .replace('\r', "\n")
            .replace('\t', "    ")
            .replace("CREATE PROCEDURE", "ALTER PROCEDURE");

        Ok(normalized)
    }

    /// Get table row count estimate
    pub async fn get_table_row_count(
        client: &mut Client<Compat<TcpStream>>,
        schema: &str,
        table: &str,
    ) -> Result<i64> {
        let query = format!(
            "SELECT SUM(p.rows) as row_count
             FROM sys.partitions p
             INNER JOIN sys.tables t ON p.object_id = t.object_id
             INNER JOIN sys.schemas s ON t.schema_id = s.schema_id
             WHERE s.name = '{}' AND t.name = '{}' AND p.index_id IN (0, 1)",
            schema, table
        );

        let stream = client.simple_query(&query).await?;
        let row = stream.into_row().await?.context("No row count")?;
        let count = row.get::<i64, _>(0).unwrap_or(0);

        Ok(count)
    }

    /// Get table DDL
    pub async fn get_table_ddl(
        client: &mut Client<Compat<TcpStream>>,
        schema: &str,
        table: &str,
    ) -> Result<String> {
        let columns = Self::get_columns(client, schema, table).await?;

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

    /// Search for objects by name
    pub async fn search_objects(
        client: &mut Client<Compat<TcpStream>>,
        search_term: &str,
    ) -> Result<Vec<DatabaseObject>> {
        let query = format!(
            "SELECT s.name as schema_name, o.name as object_name, o.type_desc
             FROM sys.objects o
             INNER JOIN sys.schemas s ON o.schema_id = s.schema_id
             WHERE o.name LIKE '%{}%' AND o.type IN ('U', 'V', 'P', 'FN', 'IF', 'TF')
             ORDER BY o.type, s.name, o.name",
            search_term
        );

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

                objects.push(DatabaseObject {
                    name,
                    schema,
                    object_type,
                });
            }
        }

        Ok(objects)
    }
}
