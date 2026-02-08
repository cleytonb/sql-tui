//! SQL context extraction for autocomplete
//!
//! Analyzes the SQL query text up to the cursor position to determine
//! what kind of completions should be offered.

/// Table reference found in query (for column suggestions)
#[derive(Clone, Debug, PartialEq)]
pub struct TableRef {
    pub schema: Option<String>,
    pub table: String,
    pub alias: Option<String>,
}

/// The SQL context at the cursor position
#[derive(Clone, Debug, PartialEq)]
pub enum SqlContext {
    /// After "schema." - suggest tables/views/procedures from that schema
    /// Example: SELECT * FROM pmt.|
    AfterSchemaDot {
        schema: String,
        object_hint: ObjectHint,
    },

    /// After "alias." or "table." - suggest columns from that table
    /// Example: SELECT c.| FROM pmt.Customers c
    AfterTableAliasDot {
        alias: String,
        table_ref: Option<TableRef>,
    },

    /// After EXEC or EXECUTE - suggest stored procedures
    /// Example: EXEC |
    AfterExec,

    /// After a table/view name - suggest WHERE, ORDER BY, JOIN, etc.
    /// Example: SELECT * FROM Customers |
    AfterTableName,

    /// After SELECT - suggest columns, *, expressions
    /// Example: SELECT |
    AfterSelect {
        tables: Vec<TableRef>,
    },

    /// After WHERE or AND/OR - suggest column names from referenced tables
    /// Example: WHERE |
    AfterWhere {
        tables: Vec<TableRef>,
    },

    /// After INSERT INTO table( - suggest columns for insert
    /// Example: INSERT INTO pmt.Contas(|
    AfterInsertIntoColumns {
        table_ref: TableRef,
    },

    /// After UPDATE table SET - suggest columns for update
    /// Example: UPDATE pmt.Contas SET |
    AfterUpdateSet {
        table_ref: TableRef,
    },

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

/// Represents the current SQL clause we're in
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum CurrentClause {
    Select,
    From,
    Join,
    Where,
    And,
    Or,
    On,
    OrderBy,
    GroupBy,
    Having,
    Set,
    Exec,
    InsertInto,
    Update,
    Unknown,
}

/// Detect which clause we're currently in based on the last significant keyword
fn detect_current_clause(upper_text: &str) -> CurrentClause {
    // List of clause keywords with their positions (keyword, position, clause type)
    let clause_keywords: &[(&str, CurrentClause)] = &[
        ("SELECT ", CurrentClause::Select),
        ("FROM ", CurrentClause::From),
        ("INNER JOIN ", CurrentClause::Join),
        ("LEFT JOIN ", CurrentClause::Join),
        ("RIGHT JOIN ", CurrentClause::Join),
        ("FULL JOIN ", CurrentClause::Join),
        ("CROSS JOIN ", CurrentClause::Join),
        ("JOIN ", CurrentClause::Join),
        ("WHERE ", CurrentClause::Where),
        (" AND ", CurrentClause::And),
        (" OR ", CurrentClause::Or),
        (" ON ", CurrentClause::On),
        ("ORDER BY ", CurrentClause::OrderBy),
        ("GROUP BY ", CurrentClause::GroupBy),
        ("HAVING ", CurrentClause::Having),
        ("SET ", CurrentClause::Set),
        ("EXEC ", CurrentClause::Exec),
        ("EXECUTE ", CurrentClause::Exec),
        ("INSERT INTO ", CurrentClause::InsertInto),
        ("UPDATE ", CurrentClause::Update),
    ];
    
    // Find the last occurrence of each keyword
    let mut last_pos: Option<usize> = None;
    let mut last_clause = CurrentClause::Unknown;
    
    for (keyword, clause) in clause_keywords {
        if let Some(pos) = upper_text.rfind(keyword) {
            if last_pos.is_none() || pos > last_pos.unwrap() {
                last_pos = Some(pos);
                last_clause = *clause;
            }
        }
    }
    
    last_clause
}

/// Check for dot contexts (schema.| or alias.|)
fn check_dot_context(before_cursor: &str, before_upper: &str, tables: &[TableRef]) -> Option<SqlContext> {
    // Check if we're right after a dot
    if before_cursor.ends_with('.') {
        if let Some(word_before) = extract_word_before_dot(before_cursor) {
            return Some(resolve_dot_context(&word_before, before_upper, tables));
        }
    }
    
    // Check if there's a dot with partial text after it (alias.col| or schema.Tab|)
    if let Some(dot_pos) = before_cursor.rfind('.') {
        let after_dot = &before_cursor[dot_pos + 1..];
        // If there's text after the dot (no spaces), we're still in dot context
        if !after_dot.is_empty() && !after_dot.contains(char::is_whitespace) {
            if let Some(word_before) = extract_word_before_dot(&before_cursor[..=dot_pos]) {
                return Some(resolve_dot_context(&word_before, before_upper, tables));
            }
        }
    }
    
    None
}

/// Resolve what kind of dot context this is (schema or alias/table)
fn resolve_dot_context(word_before: &str, before_upper: &str, tables: &[TableRef]) -> SqlContext {
    // Check if this is a table alias
    if let Some(table_ref) = find_table_by_alias(tables, word_before) {
        return SqlContext::AfterTableAliasDot {
            alias: word_before.to_string(),
            table_ref: Some(table_ref),
        };
    }
    
    // Check if it's a known table name
    if tables.iter().any(|t| t.table.eq_ignore_ascii_case(word_before)) {
        let table_ref = tables.iter()
            .find(|t| t.table.eq_ignore_ascii_case(word_before))
            .cloned();
        return SqlContext::AfterTableAliasDot {
            alias: word_before.to_string(),
            table_ref,
        };
    }
    
    // Otherwise, treat as schema
    let object_hint = if contains_exec_context(before_upper) {
        ObjectHint::Procedure
    } else {
        ObjectHint::TableOrView
    };
    
    SqlContext::AfterSchemaDot {
        schema: word_before.to_string(),
        object_hint,
    }
}

/// Find the start of the current SQL statement containing the cursor.
/// Splits on statement-starting keywords (SELECT, INSERT, UPDATE, DELETE, EXEC, WITH, CREATE, ALTER, DROP)
/// that appear at the beginning of a line (after optional whitespace).
fn find_current_statement_start(query: &str, cursor_pos: usize) -> usize {
    let before = &query[..cursor_pos.min(query.len())];

    // Walk backwards through lines to find the last statement-starting keyword
    let statement_starters = [
        "SELECT", "INSERT", "UPDATE", "DELETE", "EXEC", "EXECUTE",
        "WITH", "CREATE", "ALTER", "DROP", "DECLARE", "BEGIN",
    ];

    let mut last_start = 0;
    let mut pos = 0;

    for line in before.split('\n') {
        let trimmed = line.trim_start().to_uppercase();
        for starter in &statement_starters {
            if trimmed.starts_with(starter) {
                // Check it's a word boundary (next char is space, newline, or end)
                let after = &trimmed[starter.len()..];
                if after.is_empty() || after.starts_with(char::is_whitespace) || after.starts_with('(') {
                    last_start = pos;
                }
            }
        }
        pos += line.len() + 1; // +1 for the '\n'
    }

    last_start
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

    // Isolate the current SQL statement to avoid mixing table refs from different statements
    let stmt_start = find_current_statement_start(query, cursor_pos);
    let current_statement = &query[stmt_start..];

    // Extract tables referenced only in the current statement
    let tables = extract_table_references(current_statement);
    
    // === PRIORITY 0: INSERT INTO columns context ===
    // Check this FIRST because INSERT INTO table( should suggest columns, not dot context
    if let Some(table_ref) = extract_insert_table_in_columns(&before_upper, before_cursor) {
        return SqlContext::AfterInsertIntoColumns { table_ref };
    }
    
    // === PRIORITY 1: Dot contexts (schema.| or alias.|) ===
    // These take precedence over most other contexts
    if let Some(dot_context) = check_dot_context(before_cursor, &before_upper, &tables) {
        return dot_context;
    }
    
    // === PRIORITY 2: Detect current clause based on last significant keyword ===
    // This is the key insight: find what clause we're IN, not just what keyword we're AFTER
    let current_clause = detect_current_clause(&before_upper);
    
    match current_clause {
        CurrentClause::Exec => {
            return SqlContext::AfterExec;
        }
        CurrentClause::Select => {
            return SqlContext::AfterSelect { tables: tables.clone() };
        }
        CurrentClause::Where | CurrentClause::And | CurrentClause::Or | 
        CurrentClause::On | CurrentClause::Having => {
            // All these expect column names
            return SqlContext::AfterWhere { tables: tables.clone() };
        }
        CurrentClause::From | CurrentClause::Join => {
            // Check if we already have a table name (then suggest clauses)
            if is_after_table_name(&before_upper, trimmed) {
                return SqlContext::AfterTableName;
            }
            // Otherwise still typing table name - fall through to General
        }
        CurrentClause::OrderBy | CurrentClause::GroupBy => {
            // These also expect column names
            return SqlContext::AfterWhere { tables: tables.clone() };
        }
        CurrentClause::Set => {
            // SET expects column names - check if we're in UPDATE context
            if let Some(table_ref) = extract_update_table(&before_upper, before_cursor) {
                return SqlContext::AfterUpdateSet { table_ref };
            }
            // Fallback to AfterWhere if we can't extract the table
            return SqlContext::AfterWhere { tables: tables.clone() };
        }
        CurrentClause::InsertInto => {
            // INSERT column list is checked at priority 0, so if we're here
            // we're still typing table name - fall through to General
        }
        CurrentClause::Update => {
            // Still typing table name after UPDATE, fall through to General
        }
        CurrentClause::Unknown => {
            // Fall through to General
        }
    }
    
    // 7. Default: general context with current prefix
    let prefix = extract_current_word(before_cursor);
    SqlContext::General { prefix }
}

/// Extract table from INSERT INTO table( context
/// Returns Some(TableRef) if we're inside the column list parentheses
fn extract_insert_table_in_columns(upper_text: &str, original_text: &str) -> Option<TableRef> {
    // Pattern: INSERT INTO [schema.]table (
    // We need to find the table name and check if we're inside ( )
    
    // Find "INSERT INTO " position
    let insert_pos = upper_text.rfind("INSERT INTO ")?;
    let after_insert = insert_pos + "INSERT INTO ".len();
    
    // Check if there's an open parenthesis after the table name
    let text_after = &original_text[after_insert..];
    let paren_pos = text_after.find('(')?;
    
    // Extract the table name (between INSERT INTO and the parenthesis)
    let table_part = text_after[..paren_pos].trim();
    if table_part.is_empty() {
        return None;
    }
    
    // Check if we haven't closed the parenthesis yet (still inside column list)
    let after_paren = &text_after[paren_pos + 1..];
    if after_paren.contains(')') {
        // Already closed, not in column list anymore
        return None;
    }
    
    // Parse the table reference
    parse_simple_table_reference(table_part)
}

/// Extract table from UPDATE table SET context
fn extract_update_table(upper_text: &str, original_text: &str) -> Option<TableRef> {
    // Pattern: UPDATE [schema.]table SET
    
    // Find "UPDATE " position
    let update_pos = upper_text.rfind("UPDATE ")?;
    let after_update = update_pos + "UPDATE ".len();
    
    // Find "SET " after UPDATE
    let set_pos = upper_text[after_update..].find(" SET ")?;
    
    // Extract table name between UPDATE and SET
    let table_part = original_text[after_update..after_update + set_pos].trim();
    if table_part.is_empty() {
        return None;
    }
    
    // Parse the table reference
    parse_simple_table_reference(table_part)
}

/// Parse a simple table reference like "schema.table" or "table"
fn parse_simple_table_reference(text: &str) -> Option<TableRef> {
    let text = text.trim().trim_matches(|c| c == '[' || c == ']');
    if text.is_empty() {
        return None;
    }
    
    // Check for schema.table format
    if let Some(dot_pos) = text.find('.') {
        let schema = text[..dot_pos].trim().trim_matches(|c| c == '[' || c == ']');
        let table = text[dot_pos + 1..].trim().trim_matches(|c| c == '[' || c == ']');
        Some(TableRef {
            schema: Some(schema.to_string()),
            table: table.to_string(),
            alias: None,
        })
    } else {
        Some(TableRef {
            schema: None,
            table: text.to_string(),
            alias: None,
        })
    }
}

/// Extract table references from the query (FROM and JOIN clauses)
fn extract_table_references(query: &str) -> Vec<TableRef> {
    let mut tables = Vec::new();
    
    // Normalize whitespace: replace all whitespace sequences with single spaces
    // This handles cases where FROM/JOIN is followed by newlines instead of spaces
    let normalized: String = query.chars()
        .map(|c| if c.is_whitespace() { ' ' } else { c })
        .collect();
    let upper = normalized.to_uppercase();
    
    // Collapse multiple spaces into one for easier matching
    let mut collapsed = String::with_capacity(upper.len());
    let mut prev_space = false;
    for c in upper.chars() {
        if c == ' ' {
            if !prev_space {
                collapsed.push(c);
            }
            prev_space = true;
        } else {
            collapsed.push(c);
            prev_space = false;
        }
    }
    
    // Also need a collapsed version of the original query (preserving case)
    let mut normalized_collapsed = String::with_capacity(normalized.len());
    prev_space = false;
    for c in normalized.chars() {
        if c == ' ' {
            if !prev_space {
                normalized_collapsed.push(c);
            }
            prev_space = true;
        } else {
            normalized_collapsed.push(c);
            prev_space = false;
        }
    }
    
    // Simple regex-like parsing for FROM and JOIN clauses
    // Pattern: FROM|JOIN schema.table [AS] alias
    //          FROM|JOIN table [AS] alias
    
    let keywords = ["FROM ", "JOIN ", "INNER JOIN ", "LEFT JOIN ", "RIGHT JOIN ", "FULL JOIN ", "CROSS JOIN "];
    
    for keyword in keywords {
        let mut search_pos = 0;
        while let Some(kw_pos) = collapsed[search_pos..].find(keyword) {
            let start = search_pos + kw_pos + keyword.len();
            if let Some(table_ref) = parse_table_reference(&normalized_collapsed[start..]) {
                // Avoid duplicates
                if !tables.iter().any(|t: &TableRef| {
                    t.table.eq_ignore_ascii_case(&table_ref.table) && t.schema == table_ref.schema
                }) {
                    tables.push(table_ref);
                }
            }
            search_pos = start;
        }
    }
    
    tables
}

/// Parse a table reference from text like "schema.table alias" or "table AS alias"
fn parse_table_reference(text: &str) -> Option<TableRef> {
    let text = text.trim();
    if text.is_empty() {
        return None;
    }
    
    // Split by whitespace to get tokens
    let tokens: Vec<&str> = text.split_whitespace().take(4).collect();
    if tokens.is_empty() {
        return None;
    }
    
    let table_part = tokens[0];
    
    // Check for schema.table format
    let (schema, table) = if let Some(dot_pos) = table_part.find('.') {
        let schema = table_part[..dot_pos].trim_matches(|c| c == '[' || c == ']');
        let table = table_part[dot_pos + 1..].trim_matches(|c| c == '[' || c == ']');
        (Some(schema.to_string()), table.to_string())
    } else {
        let table = table_part.trim_matches(|c| c == '[' || c == ']');
        (None, table.to_string())
    };
    
    // Stop if we hit a keyword
    let stop_keywords = ["WHERE", "ORDER", "GROUP", "HAVING", "ON", "SET", "VALUES", "(", ")", ","];
    if stop_keywords.contains(&table.to_uppercase().as_str()) {
        return None;
    }
    
    // Look for alias (second token, or after AS)
    let alias = if tokens.len() > 1 {
        let next = tokens[1].to_uppercase();
        if next == "AS" && tokens.len() > 2 {
            // Skip AS, get the alias
            let alias = tokens[2].trim_matches(|c| c == '[' || c == ']');
            if !stop_keywords.contains(&alias.to_uppercase().as_str()) {
                Some(alias.to_string())
            } else {
                None
            }
        } else if !stop_keywords.contains(&next.as_str()) 
            && !["WHERE", "ORDER", "GROUP", "HAVING", "ON", "INNER", "LEFT", "RIGHT", "FULL", "CROSS", "JOIN", "SET", "AND", "OR"].contains(&next.as_str()) 
        {
            let alias = tokens[1].trim_matches(|c| c == '[' || c == ']');
            Some(alias.to_string())
        } else {
            None
        }
    } else {
        None
    };
    
    Some(TableRef { schema, table, alias })
}

/// Find a table reference by its alias
fn find_table_by_alias(tables: &[TableRef], alias: &str) -> Option<TableRef> {
    tables.iter()
        .find(|t| {
            t.alias.as_ref().map(|a| a.eq_ignore_ascii_case(alias)).unwrap_or(false)
        })
        .cloned()
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

    #[test]
    fn test_alias_after_from_with_newline() {
        // FROM followed by newline instead of space
        let query = "SELECT\n    c.\nFROM\n    pmt.Contas c";
        let ctx = extract_context(query, 13); // Position after "c."
        
        assert!(matches!(
            &ctx,
            SqlContext::AfterTableAliasDot { alias, table_ref: Some(tref) } 
            if alias == "c" && tref.table == "Contas"
        ), "Expected AfterTableAliasDot for alias 'c', got {:?}", ctx);
    }

    #[test]
    fn test_multiple_aliases_with_newlines() {
        // Realistic query with multiple table aliases - cursor after nc.
        // "SELECT\n    c.Nome,\n    nc." = 25 chars
        let query = "SELECT\n    c.Nome,\n    nc.\nFROM\n    pmt.Contas c\njoin pmt.NegociacoesContas nc ON c.CodNegociacaoConta = nc.CodNegociacaoConta";
        
        // Find the position after "nc."
        let cursor_pos = query.find("nc.").unwrap() + 3;
        
        // Check alias 'nc' is resolved
        let ctx = extract_context(query, cursor_pos);
        assert!(matches!(
            &ctx,
            SqlContext::AfterTableAliasDot { alias, table_ref: Some(tref) } 
            if alias == "nc" && tref.table == "NegociacoesContas"
        ), "Expected AfterTableAliasDot for alias 'nc', got {:?}", ctx);
    }

    #[test]
    fn test_extract_table_refs_from_with_newline() {
        let query = "SELECT * FROM\n    pmt.Contas c\njoin pmt.NegociacoesContas nc ON x = y";
        let tables = extract_table_references(query);
        
        assert_eq!(tables.len(), 2, "Expected 2 tables, got {:?}", tables);
        
        // First table: pmt.Contas c
        let contas = tables.iter().find(|t| t.table == "Contas");
        assert!(contas.is_some(), "Expected to find Contas table");
        let contas = contas.unwrap();
        assert_eq!(contas.alias, Some("c".to_string()));
        assert_eq!(contas.schema, Some("pmt".to_string()));
        
        // Second table: pmt.NegociacoesContas nc
        let neg = tables.iter().find(|t| t.table == "NegociacoesContas");
        assert!(neg.is_some(), "Expected to find NegociacoesContas table");
        let neg = neg.unwrap();
        assert_eq!(neg.alias, Some("nc".to_string()));
        assert_eq!(neg.schema, Some("pmt".to_string()));
    }

    #[test]
    fn test_insert_into_columns_context() {
        // INSERT INTO pmt.Contas(| - should suggest columns
        let query = "INSERT INTO pmt.Contas(";
        let ctx = extract_context(query, query.len());
        assert!(matches!(
            &ctx,
            SqlContext::AfterInsertIntoColumns { table_ref } 
            if table_ref.table == "Contas" && table_ref.schema == Some("pmt".to_string())
        ), "Expected AfterInsertIntoColumns, got {:?}", ctx);
    }

    #[test]
    fn test_insert_into_columns_partial() {
        // INSERT INTO Contas(Nome, | - still in column list
        let query = "INSERT INTO Contas(Nome, ";
        let ctx = extract_context(query, query.len());
        assert!(matches!(
            &ctx,
            SqlContext::AfterInsertIntoColumns { table_ref } 
            if table_ref.table == "Contas"
        ), "Expected AfterInsertIntoColumns, got {:?}", ctx);
    }

    #[test]
    fn test_insert_into_columns_closed() {
        // INSERT INTO Contas(Nome) VALUES (| - parenthesis closed, not in column list
        let query = "INSERT INTO Contas(Nome) VALUES (";
        let ctx = extract_context(query, query.len());
        // Should NOT be AfterInsertIntoColumns because parens are closed
        assert!(!matches!(ctx, SqlContext::AfterInsertIntoColumns { .. }), 
            "Should not be AfterInsertIntoColumns after closing parens, got {:?}", ctx);
    }

    #[test]
    fn test_update_set_context() {
        // UPDATE pmt.Contas SET | - should suggest columns
        let query = "UPDATE pmt.Contas SET ";
        let ctx = extract_context(query, query.len());
        assert!(matches!(
            &ctx,
            SqlContext::AfterUpdateSet { table_ref } 
            if table_ref.table == "Contas" && table_ref.schema == Some("pmt".to_string())
        ), "Expected AfterUpdateSet, got {:?}", ctx);
    }

    #[test]
    fn test_update_set_with_alias() {
        // UPDATE Contas SET Nome = | - still in SET context
        let query = "UPDATE Contas SET Nome = 'teste', ";
        let ctx = extract_context(query, query.len());
        assert!(matches!(
            &ctx,
            SqlContext::AfterUpdateSet { table_ref } 
            if table_ref.table == "Contas"
        ), "Expected AfterUpdateSet, got {:?}", ctx);
    }

    #[test]
    fn test_multiple_statements_isolates_current() {
        // Two separate SELECT statements - cursor is in the second one after "c."
        // Should resolve alias 'c' to Chargebacks, NOT Contas
        let query = "SELECT *\nFROM pmt.Contas c\nWHERE c.Ativo = 1\n\nSELECT *\nFROM pmt.Chargebacks c\nWHERE c.";
        let cursor_pos = query.len();
        let ctx = extract_context(query, cursor_pos);
        assert!(matches!(
            &ctx,
            SqlContext::AfterTableAliasDot { alias, table_ref: Some(tref) }
            if alias == "c" && tref.table == "Chargebacks"
        ), "Expected AfterTableAliasDot for Chargebacks, got {:?}", ctx);
    }
}
