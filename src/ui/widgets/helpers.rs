//! Helper functions for UI widgets

use crate::db::CellValue;

/// Get type indicator emoji for column type
pub fn get_type_indicator(type_name: &str) -> &'static str {
    match type_name.to_uppercase().as_str() {
        "INT" | "INTEGER" | "BIGINT" | "SMALLINT" | "TINYINT" => "🔢",
        "DECIMAL" | "NUMERIC" | "FLOAT" | "REAL" | "MONEY" | "SMALLMONEY" => "💰",
        "VARCHAR" | "NVARCHAR" | "CHAR" | "NCHAR" | "TEXT" | "NTEXT" | "VARCHAR(MAX)" => "📝",
        "DATETIME" | "DATETIME2" | "DATE" | "TIME" | "DATETIMEOFFSET" | "SMALLDATETIME" => "📅",
        "BIT" => "✓",
        "BINARY" | "VARBINARY" | "VARBINARY(MAX)" | "IMAGE" => "📦",
        "UNIQUEIDENTIFIER" => "🔑",
        "XML" => "📄",
        _ => "•",
    }
}

/// Format cell value for display with NULL handling
pub fn format_cell_value(cell: &CellValue) -> (String, bool) {
    match cell {
        CellValue::Null => ("NULL".to_string(), true),
        CellValue::Bool(v) => (if *v { "✓ true" } else { "✗ false" }.to_string(), false),
        CellValue::Int(v) => (format_number(*v), false),
        CellValue::Float(v) => (format!("{:.4}", v), false),
        CellValue::String(v) => {
            // Truncate long strings
            if v.len() > 50 {
                (format!("{}…", &v[..47]), false)
            } else {
                (v.clone(), false)
            }
        }
        CellValue::DateTime(v) => (v.clone(), false),
        CellValue::Binary(v) => (format!("0x{}…", &hex_encode(&v[..v.len().min(8)])), false),
    }
}

/// Format number with thousand separators
pub fn format_number(n: i64) -> String {
    let s = n.abs().to_string();
    let mut result = String::new();
    for (i, c) in s.chars().rev().enumerate() {
        if i > 0 && i % 3 == 0 {
            result.push(',');
        }
        result.push(c);
    }
    if n < 0 {
        result.push('-');
    }
    result.chars().rev().collect()
}

/// Hex encode bytes
pub fn hex_encode(data: &[u8]) -> String {
    data.iter().map(|b| format!("{:02X}", b)).collect()
}
