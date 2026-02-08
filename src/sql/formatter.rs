//! SQL formatter - disabled

/// Format SQL query (currently disabled - returns input as-is)
pub fn format_sql_query(sql: &str) -> String {
    sql.to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_returns_input() {
        let sql = "select * from users where id = 1";
        let formatted = format_sql_query(sql);
        assert_eq!(sql, formatted);
    }
}
