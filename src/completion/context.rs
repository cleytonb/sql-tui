//! SQL context extraction for autocomplete
//!
//! Analyzes the SQL query text up to the cursor position to determine
//! what kind of completions should be offered.

/// The SQL context at the cursor position
#[derive(Clone, Debug, PartialEq)]
pub enum SqlContext {
    /// After "schema." - suggest tables/views/procedures from that schema
    /// Example: SELECT * FROM pmt.|
    AfterSchemaDot {
        schema: String,
        object_hint: ObjectHint,
    },

    /// After EXEC or EXECUTE - suggest stored procedures
    /// Example: EXEC |
    AfterExec,

    /// After a table/view name - suggest WHERE, ORDER BY, JOIN, etc.
    /// Example: SELECT * FROM Customers |
    AfterTableName,

    /// After SELECT - suggest columns, *, expressions
    /// Example: SELECT |
    AfterSelect,

    /// After WHERE or AND/OR - suggest column names
    /// Example: WHERE |
    AfterWhere,

    /// General context - suggest keywords and all objects
    General {
        prefix: String,
    },
}

/// Hint about what type of object is expected
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ObjectHint {
    /// Tables or views (after FROM, JOIN)
    TableOrView,
    /// Stored procedures (after EXEC)
    Procedure,
    /// Any object type
    Any,
}

/// Extract the SQL context at the given cursor position
pub fn extract_context(query: &str, cursor_pos: usize) -> SqlContext {
    let before_cursor = if cursor_pos <= query.len() {
        &query[..cursor_pos]
    } else {
        query
    };
    
    let before_upper = before_cursor.to_uppercase();
    let trimmed = before_cursor.trim_end();
    
    // 1. Check if we're right after a dot (schema.)
    if before_cursor.ends_with('.') {
        if let Some(schema) = extract_word_before_dot(before_cursor) {
            // Determine object hint based on context
            let object_hint = if contains_exec_context(&before_upper) {
                ObjectHint::Procedure
            } else {
                ObjectHint::TableOrView
            };
            
            return SqlContext::AfterSchemaDot {
                schema,
                object_hint,
            };
        }
    }
    
    // 2. Check if there's a dot with partial text after it (schema.Tab|)
    if let Some(dot_pos) = before_cursor.rfind('.') {
        let after_dot = &before_cursor[dot_pos + 1..];
        // If there's text after the dot (no spaces), we're still in schema context
        if !after_dot.is_empty() && !after_dot.contains(char::is_whitespace) {
            if let Some(schema) = extract_word_before_dot(&before_cursor[..=dot_pos]) {
                let object_hint = if contains_exec_context(&before_upper) {
                    ObjectHint::Procedure
                } else {
                    ObjectHint::TableOrView
                };
                
                return SqlContext::AfterSchemaDot {
                    schema,
                    object_hint,
                };
            }
        }
    }
    
    // 3. Check for EXEC/EXECUTE followed by space
    if before_upper.ends_with("EXEC ") || before_upper.ends_with("EXECUTE ") {
        return SqlContext::AfterExec;
    }
    
    // Also check if we're typing after EXEC with partial text
    if let Some(exec_pos) = find_last_keyword(&before_upper, &["EXEC ", "EXECUTE "]) {
        let after_exec = &before_cursor[exec_pos..].trim();
        // If no dot yet, still in EXEC context
        if !after_exec.contains('.') && !after_exec.contains(char::is_whitespace) {
            return SqlContext::AfterExec;
        }
    }
    
    // 4. Check for SELECT without FROM yet
    if before_upper.contains("SELECT") && !before_upper.contains("FROM") {
        // Check if we're right after SELECT
        if before_upper.ends_with("SELECT ") {
            return SqlContext::AfterSelect;
        }
    }
    
    // 5. Check for WHERE context
    if before_upper.ends_with("WHERE ") 
        || before_upper.ends_with("AND ") 
        || before_upper.ends_with("OR ") 
    {
        return SqlContext::AfterWhere;
    }
    
    // 6. Check if we just finished a table name (after FROM/JOIN)
    if is_after_table_name(&before_upper, trimmed) {
        return SqlContext::AfterTableName;
    }
    
    // 7. Default: general context with current prefix
    let prefix = extract_current_word(before_cursor);
    SqlContext::General { prefix }
}

/// Extract the word immediately before a dot
fn extract_word_before_dot(text: &str) -> Option<String> {
    let text = text.trim_end_matches('.');
    let chars: Vec<char> = text.chars().collect();
    
    if chars.is_empty() {
        return None;
    }
    
    let end = chars.len();
    let mut start = end;
    
    // Walk backwards to find word boundaries
    for i in (0..chars.len()).rev() {
        let c = chars[i];
        if c.is_alphanumeric() || c == '_' {
            start = i;
        } else {
            break;
        }
    }
    
    if start < end {
        Some(chars[start..end].iter().collect())
    } else {
        None
    }
}

/// Extract the current word being typed (for filtering)
fn extract_current_word(text: &str) -> String {
    let chars: Vec<char> = text.chars().collect();
    
    if chars.is_empty() {
        return String::new();
    }
    
    let mut start = chars.len();
    
    // Walk backwards to find word start
    for i in (0..chars.len()).rev() {
        let c = chars[i];
        if c.is_alphanumeric() || c == '_' {
            start = i;
        } else {
            break;
        }
    }
    
    chars[start..].iter().collect()
}

/// Check if text contains EXEC context
fn contains_exec_context(upper_text: &str) -> bool {
    // Find the last occurrence of EXEC or EXECUTE
    let exec_pos = upper_text.rfind("EXEC ");
    let execute_pos = upper_text.rfind("EXECUTE ");
    
    // Check if FROM/SELECT came after EXEC (which would mean we're not in EXEC context)
    let from_pos = upper_text.rfind("FROM ");
    let select_pos = upper_text.rfind("SELECT ");
    
    match (exec_pos.or(execute_pos), from_pos.max(select_pos)) {
        (Some(exec), Some(other)) => exec > other,
        (Some(_), None) => true,
        _ => false,
    }
}

/// Find the position after the last occurrence of any of the given keywords
fn find_last_keyword(text: &str, keywords: &[&str]) -> Option<usize> {
    keywords
        .iter()
        .filter_map(|kw| text.rfind(kw).map(|pos| pos + kw.len()))
        .max()
}

/// Check if cursor is right after a table name (for suggesting WHERE, JOIN, etc.)
fn is_after_table_name(upper_text: &str, trimmed: &str) -> bool {
    // Pattern: FROM something | or JOIN something |
    // where | is the cursor and something is a table reference
    
    // Simple heuristic: if we end with a space after what looks like an identifier
    // and FROM or JOIN is in the query
    
    if !trimmed.ends_with(char::is_whitespace) {
        return false;
    }
    
    let has_from_or_join = upper_text.contains("FROM ") || upper_text.contains("JOIN ");
    if !has_from_or_join {
        return false;
    }
    
    // Check if we haven't started a WHERE/ORDER yet
    let last_keyword_pos = ["WHERE ", "ORDER ", "GROUP ", "HAVING "]
        .iter()
        .filter_map(|kw| upper_text.rfind(kw))
        .max();
    
    let last_from_join = ["FROM ", "JOIN "]
        .iter()
        .filter_map(|kw| upper_text.rfind(kw))
        .max();
    
    match (last_from_join, last_keyword_pos) {
        (Some(fj), Some(kw)) => fj > kw,
        (Some(_), None) => true,
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_after_schema_dot() {
        let ctx = extract_context("SELECT * FROM pmt.", 18);
        assert!(matches!(ctx, SqlContext::AfterSchemaDot { schema, .. } if schema == "pmt"));
    }

    #[test]
    fn test_after_exec() {
        let ctx = extract_context("EXEC ", 5);
        assert!(matches!(ctx, SqlContext::AfterExec));
    }

    #[test]
    fn test_exec_schema_dot() {
        let ctx = extract_context("EXEC pmt.", 9);
        assert!(matches!(
            ctx,
            SqlContext::AfterSchemaDot { schema, object_hint: ObjectHint::Procedure } if schema == "pmt"
        ));
    }

    #[test]
    fn test_general() {
        let ctx = extract_context("SEL", 3);
        assert!(matches!(ctx, SqlContext::General { prefix } if prefix == "SEL"));
    }
}
