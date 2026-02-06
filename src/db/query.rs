//! Query execution and result handling

use anyhow::Result;
use tiberius::time::chrono::{NaiveDate, NaiveDateTime, NaiveTime};
use std::time::{Duration, Instant};
use tiberius::{Client, Column, ColumnType, Row, numeric::Numeric};
use tokio::net::TcpStream;
use tokio_util::compat::Compat;

/// Represents a cell value in the result set
#[derive(Clone, Debug)]
pub enum CellValue {
    Null,
    Bool(bool),
    Int(i64),
    Float(f64),
    String(String),
    DateTime(String),
    Binary(Vec<u8>),
}

impl std::fmt::Display for CellValue {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CellValue::Null => write!(f, "NULL"),
            CellValue::Bool(v) => write!(f, "{}", if *v { "true" } else { "false" }),
            CellValue::Int(v) => write!(f, "{}", v),
            CellValue::Float(v) => write!(f, "{:.6}", v),
            CellValue::String(v) => write!(f, "{}", v),
            CellValue::DateTime(v) => write!(f, "{}", v),
            CellValue::Binary(v) => write!(f, "0x{}", hex::encode(v)),
        }
    }
}

/// Column metadata
#[derive(Clone, Debug)]
pub struct ColumnInfo {
    pub name: String,
    pub type_name: String,
    pub max_width: usize,
}

/// Query result
#[derive(Clone, Debug)]
pub struct QueryResult {
    pub columns: Vec<ColumnInfo>,
    pub rows: Vec<Vec<CellValue>>,
    pub row_count: usize,
    pub execution_time: Duration,
    pub affected_rows: Option<u64>,
    pub messages: Vec<String>,
}

impl QueryResult {
    pub fn empty() -> Self {
        Self {
            columns: Vec::new(),
            rows: Vec::new(),
            row_count: 0,
            execution_time: Duration::ZERO,
            affected_rows: None,
            messages: Vec::new(),
        }
    }
}

/// Query executor
pub struct QueryExecutor;

impl QueryExecutor {
    /// Execute a query and return results
    pub async fn execute(
        client: &mut Client<Compat<TcpStream>>,
        query: &str,
    ) -> Result<QueryResult> {
        let start = Instant::now();

        // Execute the query
        let result = client.simple_query(query).await;

        match result {
            Ok(stream) => Self::process_results(stream, start).await,
            Err(e) => Err(e.into()),
        }
    }

    /// Process query results from a stream
    async fn process_results(
        stream: tiberius::QueryStream<'_>,
        start: Instant,
    ) -> Result<QueryResult> {
        let mut columns: Vec<ColumnInfo> = Vec::new();
        let mut rows: Vec<Vec<CellValue>> = Vec::new();

        // Process results
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

        let execution_time = start.elapsed();

        Ok(QueryResult {
            row_count: rows.len(),
            columns,
            rows,
            execution_time,
            affected_rows: None,
            messages: Vec::new(),
        })
    }

    /// Execute multiple queries
    pub async fn execute_batch(
        client: &mut Client<Compat<TcpStream>>,
        queries: &[&str],
    ) -> Result<Vec<QueryResult>> {
        let mut results = Vec::new();

        for query in queries {
            let result = Self::execute(client, query).await?;
            results.push(result);
        }

        Ok(results)
    }
}

fn format_column_type(col: &Column) -> String {
    match col.column_type() {
        ColumnType::Null => "NULL".to_string(),
        ColumnType::Bit => "BIT".to_string(),
        ColumnType::Int1 => "TINYINT".to_string(),
        ColumnType::Int2 => "SMALLINT".to_string(),
        ColumnType::Int4 => "INT".to_string(),
        ColumnType::Int8 => "BIGINT".to_string(),
        ColumnType::Float4 => "REAL".to_string(),
        ColumnType::Float8 => "FLOAT".to_string(),
        ColumnType::Datetime => "DATETIME".to_string(),
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
        ColumnType::Bit => row
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
        ColumnType::Datetime | ColumnType::Datetime2 => row
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
        // Fallback: try various types in order of likelihood
        _ => {
            // Try string first (most common)
            if let Some(v) = row.try_get::<&str, _>(index).ok().flatten() {
                return CellValue::String(v.to_string());
            }
            // Try datetime
            if let Some(v) = row.try_get::<NaiveDateTime, _>(index).ok().flatten() {
                return CellValue::DateTime(v.format("%Y-%m-%d %H:%M:%S").to_string());
            }
            // Try integer
            if let Some(v) = row.try_get::<i64, _>(index).ok().flatten() {
                return CellValue::Int(v);
            }
            // Try float
            if let Some(v) = row.try_get::<f64, _>(index).ok().flatten() {
                return CellValue::Float(v);
            }
            // Try numeric
            if let Some(v) = row.try_get::<Numeric, _>(index).ok().flatten() {
                return CellValue::String(v.to_string());
            }
            // Give up - return type info as string
            CellValue::String(format!("<{:?}>", col.column_type()))
        }
    }
}

// Helper for hex encoding binary data
mod hex {
    pub fn encode(data: &[u8]) -> String {
        data.iter().map(|b| format!("{:02X}", b)).collect()
    }
}
