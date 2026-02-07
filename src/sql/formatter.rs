//! SQL formatter - formats SQL with proper indentation and line breaks

/// Format SQL query with proper indentation and line breaks
pub fn format_sql_query(sql: &str) -> String {
    let keywords_newline_before = [
        "SELECT", "FROM", "WHERE", "AND", "OR", "ORDER BY", "GROUP BY",
        "HAVING", "JOIN", "INNER JOIN", "LEFT JOIN", "RIGHT JOIN",
        "OUTER JOIN", "CROSS JOIN", "UNION", "UNION ALL",
        "INSERT INTO", "VALUES", "UPDATE", "SET", "DELETE FROM",
        "CREATE TABLE", "ALTER TABLE", "DROP TABLE", "CROSS", "OUTER"
    ];

    let keywords_newline_after = ["SELECT"];

    // Normalize whitespace
    let sql = sql.split_whitespace().collect::<Vec<_>>().join(" ");

    let mut result = String::new();
    let mut indent_level = 0;
    let mut i = 0;
    let chars: Vec<char> = sql.chars().collect();
    let sql_upper = sql.to_uppercase();

    while i < chars.len() {
        // Check for keywords that need newline before
        let mut matched_keyword = None;
        for keyword in &keywords_newline_before {
            if sql_upper[i..].starts_with(keyword) {
                // Make sure it's a word boundary
                let end = i + keyword.len();
                if end >= sql_upper.len() || !sql_upper.chars().nth(end).unwrap().is_alphanumeric() {
                    matched_keyword = Some(*keyword);
                    break;
                }
            }
        }

        if let Some(keyword) = matched_keyword {
            // Add newline before keyword (except at start)
            if !result.is_empty() && !result.ends_with('\n') {
                result.push('\n');
            }

            // Handle indentation
            match keyword {
                "AND" | "OR" => {
                    result.push_str(&"    ".repeat(indent_level + 1));
                }
                _ => {
                    result.push_str(&"    ".repeat(indent_level));
                }
            }

            // Add the keyword with original case preserved where possible
            let original_keyword: String = chars[i..i + keyword.len()].iter().collect();
            result.push_str(&original_keyword.to_uppercase());
            i += keyword.len();

            // Add newline after certain keywords
            if keywords_newline_after.contains(&keyword) {
                result.push('\n');
                result.push_str(&"    ".repeat(indent_level + 1));
            } else {
                result.push(' ');
            }

            // Skip any following whitespace
            while i < chars.len() && chars[i].is_whitespace() {
                i += 1;
            }
        } else if chars[i] == '(' {
            result.push('(');
            indent_level += 1;
            i += 1;
        } else if chars[i] == ')' {
            result.push('\n');
            indent_level = indent_level.saturating_sub(1);
            result.push_str(&"    ".repeat(indent_level));
            result.push(')');
            i += 1;
        } else if chars[i] == ',' {
            result.push(',');
            result.push('\n');
            result.push_str(&"    ".repeat(indent_level + 1));
            i += 1;
            // Skip whitespace after comma
            while i < chars.len() && chars[i].is_whitespace() {
                i += 1;
            }
        } else {
            result.push(chars[i]);
            i += 1;
        }
    }

    // Clean up extra whitespace
    result
        .lines()
        .map(|line| line.trim_end())
        .collect::<Vec<_>>()
        .join("\n")
        .trim()
        .to_string()
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
