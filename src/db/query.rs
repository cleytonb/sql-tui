//! Shared query result types used by all database drivers

use std::time::Duration;

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

// Helper for hex encoding binary data
mod hex {
    pub fn encode(data: &[u8]) -> String {
        data.iter().map(|b| format!("{:02X}", b)).collect()
    }
}
