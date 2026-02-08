//! SQL formatter - uses sqlformat for proper SQL formatting

use sqlformat::{format, FormatOptions, Indent, QueryParams};

/// Format SQL query using sqlformat
pub fn format_sql_query(sql: &str) -> String {
    let options = FormatOptions {
        indent: Indent::Spaces(4),
        uppercase: Some(true),
        lines_between_queries: 2,
        ..Default::default()
    };
    
    format(sql, &QueryParams::None, &options)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_simple_select() {
        let sql = "SELECT * FROM users WHERE id = 1";
        let formatted = format_sql_query(sql);
        assert!(formatted.contains("SELECT"));
        assert!(formatted.contains("FROM"));
        assert!(formatted.contains("WHERE"));
    }

    #[test]
    fn test_format_preserves_content() {
        let sql = "SELECT id, name FROM users";
        let formatted = format_sql_query(sql);
        assert!(formatted.contains("id"));
        assert!(formatted.contains("name"));
        assert!(formatted.contains("users"));
    }
}
