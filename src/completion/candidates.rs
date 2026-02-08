//! Completion candidate generation
//!
//! Generates completion candidates based on SQL context and database schema.

use super::{CompletionItem, CompletionKind, SqlContext, ObjectHint, TableRef};
use crate::app::{SchemaNode, SchemaNodeType};
use crate::db::ColumnDef;
use std::collections::HashMap;

/// Generate completion candidates based on context (sync version for non-column contexts)
pub fn get_candidates(
    context: &SqlContext,
    schema_tree: &[SchemaNode],
    prefix: &str,
) -> Vec<CompletionItem> {
    // For contexts that don't need columns, use the sync version
    get_candidates_internal(context, schema_tree, prefix, &HashMap::new())
}

/// Generate completion candidates with column cache access
pub fn get_candidates_with_columns(
    context: &SqlContext,
    schema_tree: &[SchemaNode],
    prefix: &str,
    column_cache: &HashMap<(String, String), Vec<ColumnDef>>,
) -> Vec<CompletionItem> {
    get_candidates_internal(context, schema_tree, prefix, column_cache)
}

/// Internal implementation that handles all contexts
fn get_candidates_internal(
    context: &SqlContext,
    schema_tree: &[SchemaNode],
    prefix: &str,
    column_cache: &HashMap<(String, String), Vec<ColumnDef>>,
) -> Vec<CompletionItem> {
    let mut items = match context {
        SqlContext::AfterSchemaDot { schema, object_hint } => {
            find_objects_in_schema(schema_tree, schema, *object_hint)
        }
        SqlContext::AfterTableAliasDot { alias: _, table_ref } => {
            // Suggest columns from the referenced table
            if let Some(ref tref) = table_ref {
                find_columns_for_table(tref, schema_tree, column_cache)
            } else {
                Vec::new()
            }
        }
        SqlContext::AfterExec => {
            // All procedures with schema prefix
            find_all_procedures_with_schema(schema_tree)
        }
        SqlContext::AfterTableName => {
            sql_clause_keywords()
        }
        SqlContext::AfterSelect { tables } => {
            let mut items = vec![
                CompletionItem::new("*", CompletionKind::Keyword),
                CompletionItem::new("TOP", CompletionKind::Keyword),
                CompletionItem::new("DISTINCT", CompletionKind::Keyword),
            ];
            // Add columns from all referenced tables
            for table_ref in tables {
                items.extend(find_columns_for_table(table_ref, schema_tree, column_cache));
            }
            // Also add common functions
            items.extend(sql_functions());
            items
        }
        SqlContext::AfterWhere { tables } => {
            let mut items = vec![
                CompletionItem::new("AND", CompletionKind::Keyword),
                CompletionItem::new("OR", CompletionKind::Keyword),
                CompletionItem::new("NOT", CompletionKind::Keyword),
                CompletionItem::new("IN", CompletionKind::Keyword),
                CompletionItem::new("LIKE", CompletionKind::Keyword),
                CompletionItem::new("BETWEEN", CompletionKind::Keyword),
                CompletionItem::new("IS NULL", CompletionKind::Keyword),
                CompletionItem::new("IS NOT NULL", CompletionKind::Keyword),
                CompletionItem::new("EXISTS", CompletionKind::Keyword),
            ];
            // Add columns from all referenced tables
            for table_ref in tables {
                items.extend(find_columns_for_table(table_ref, schema_tree, column_cache));
            }
            items
        }
        SqlContext::General { prefix: _ } => {
            let mut items = sql_keywords();
            items.extend(find_all_objects(schema_tree));
            items
        }
    };
    
    // Filter by prefix if provided
    if !prefix.is_empty() {
        let prefix_lower = prefix.to_lowercase();
        items.retain(|item| {
            item.label.to_lowercase().starts_with(&prefix_lower)
        });
    }
    
    // Sort: columns first when in column context, then keywords, then by label
    items.sort_by(|a, b| {
        match (&a.kind, &b.kind) {
            (CompletionKind::Column, CompletionKind::Column) => a.label.cmp(&b.label),
            (CompletionKind::Column, _) => std::cmp::Ordering::Less,
            (_, CompletionKind::Column) => std::cmp::Ordering::Greater,
            (CompletionKind::Keyword, CompletionKind::Keyword) => a.label.cmp(&b.label),
            (CompletionKind::Keyword, _) => std::cmp::Ordering::Less,
            (_, CompletionKind::Keyword) => std::cmp::Ordering::Greater,
            _ => a.label.cmp(&b.label),
        }
    });
    
    items
}

/// Find columns for a specific table reference using the cache
fn find_columns_for_table(
    table_ref: &TableRef,
    schema_tree: &[SchemaNode],
    column_cache: &HashMap<(String, String), Vec<ColumnDef>>,
) -> Vec<CompletionItem> {
    let mut items = Vec::new();
    
    // If we have schema info, use it directly
    if let Some(ref schema) = table_ref.schema {
        let key = (schema.clone(), table_ref.table.clone());
        if let Some(columns) = column_cache.get(&key) {
            for col in columns {
                let detail = format!("{} ({})", col.data_type, if col.is_nullable { "NULL" } else { "NOT NULL" });
                items.push(CompletionItem {
                    label: col.name.clone(),
                    kind: CompletionKind::Column,
                    insert_text: col.name.clone(),
                    detail: Some(detail),
                });
            }
            return items;
        }
    }
    
    // If no schema provided, search all schemas for this table
    for root_folder in schema_tree {
        if root_folder.name != "Tables" && root_folder.name != "Views" {
            continue;
        }
        
        for schema_folder in &root_folder.children {
            let schema_name = &schema_folder.name;
            
            for obj in &schema_folder.children {
                if obj.name.eq_ignore_ascii_case(&table_ref.table) {
                    let key = (schema_name.clone(), obj.name.clone());
                    if let Some(columns) = column_cache.get(&key) {
                        for col in columns {
                            let detail = format!("{} ({})", col.data_type, if col.is_nullable { "NULL" } else { "NOT NULL" });
                            items.push(CompletionItem {
                                label: col.name.clone(),
                                kind: CompletionKind::Column,
                                insert_text: col.name.clone(),
                                detail: Some(detail),
                            });
                        }
                        return items;
                    }
                }
            }
        }
    }
    
    items
}

/// Find objects in a specific schema
fn find_objects_in_schema(
    schema_tree: &[SchemaNode],
    schema_name: &str,
    hint: ObjectHint,
) -> Vec<CompletionItem> {
    let mut items = Vec::new();
    let schema_lower = schema_name.to_lowercase();
    
    for root_folder in schema_tree {
        // Filter by folder type based on hint
        let should_search = match hint {
            ObjectHint::TableOrView => {
                root_folder.name == "Tables" || root_folder.name == "Views"
            }
            ObjectHint::Procedure => {
                root_folder.name == "Stored Procedures"
            }
            ObjectHint::Any => true,
        };
        
        if !should_search {
            continue;
        }
        
        // Look for schema subfolder
        for schema_folder in &root_folder.children {
            if schema_folder.name.to_lowercase() == schema_lower {
                // Add all objects in this schema
                for obj in &schema_folder.children {
                    let kind = match obj.node_type {
                        SchemaNodeType::Table => CompletionKind::Table,
                        SchemaNodeType::View => CompletionKind::View,
                        SchemaNodeType::Procedure => CompletionKind::Procedure,
                        SchemaNodeType::Function => CompletionKind::Function,
                        _ => continue,
                    };
                    
                    items.push(CompletionItem::new(&obj.name, kind));
                }
            }
        }
    }
    
    items
}

/// Find all procedures with schema.name format
fn find_all_procedures_with_schema(schema_tree: &[SchemaNode]) -> Vec<CompletionItem> {
    let mut items = Vec::new();
    
    for root_folder in schema_tree {
        if root_folder.name != "Stored Procedures" {
            continue;
        }
        
        for schema_folder in &root_folder.children {
            let schema_name = &schema_folder.name;
            
            for proc in &schema_folder.children {
                if proc.node_type == SchemaNodeType::Procedure {
                    let full_name = format!("{}.{}", schema_name, proc.name);
                    items.push(CompletionItem::with_schema(
                        &full_name,
                        CompletionKind::Procedure,
                        schema_name,
                    ));
                }
            }
        }
    }
    
    items
}

/// Find all objects (tables, views, procedures)
fn find_all_objects(schema_tree: &[SchemaNode]) -> Vec<CompletionItem> {
    let mut items = Vec::new();
    
    for root_folder in schema_tree {
        for schema_folder in &root_folder.children {
            let schema_name = &schema_folder.name;
            
            for obj in &schema_folder.children {
                let kind = match obj.node_type {
                    SchemaNodeType::Table => CompletionKind::Table,
                    SchemaNodeType::View => CompletionKind::View,
                    SchemaNodeType::Procedure => CompletionKind::Procedure,
                    SchemaNodeType::Function => CompletionKind::Function,
                    _ => continue,
                };
                
                // Use schema.name format
                let full_name = format!("{}.{}", schema_name, obj.name);
                items.push(CompletionItem::with_schema(
                    &full_name,
                    kind,
                    schema_name,
                ));
            }
        }
    }
    
    items
}

/// SQL keywords for completion
fn sql_keywords() -> Vec<CompletionItem> {
    let keywords = [
        "SELECT", "FROM", "WHERE", "AND", "OR", "NOT", "IN", "LIKE", "BETWEEN",
        "ORDER BY", "ASC", "DESC", "GROUP BY", "HAVING", "JOIN", "INNER JOIN",
        "LEFT JOIN", "RIGHT JOIN", "FULL JOIN", "CROSS JOIN", "ON", "AS",
        "DISTINCT", "TOP", "WITH", "INSERT INTO", "VALUES", "UPDATE", "SET",
        "DELETE", "CREATE TABLE", "ALTER TABLE", "DROP TABLE", "CREATE INDEX",
        "CREATE VIEW", "CREATE PROCEDURE", "BEGIN", "END", "IF", "ELSE",
        "WHILE", "RETURN", "DECLARE", "EXEC", "EXECUTE", "NULL", "IS NULL",
        "IS NOT NULL", "CASE", "WHEN", "THEN", "UNION", "UNION ALL", "EXISTS",
        "COALESCE", "ISNULL", "CAST", "CONVERT",
    ];
    
    keywords
        .iter()
        .map(|kw| CompletionItem::new(*kw, CompletionKind::Keyword))
        .collect()
}

/// SQL functions for completion
fn sql_functions() -> Vec<CompletionItem> {
    let functions = [
        "COUNT", "SUM", "AVG", "MIN", "MAX", "LEN", "SUBSTRING", "UPPER",
        "LOWER", "TRIM", "LTRIM", "RTRIM", "REPLACE", "CONCAT", "GETDATE",
        "DATEADD", "DATEDIFF", "YEAR", "MONTH", "DAY", "ISNULL", "COALESCE",
        "CAST", "CONVERT", "ROW_NUMBER", "RANK", "DENSE_RANK", "LAG", "LEAD",
    ];
    
    functions
        .iter()
        .map(|f| CompletionItem::new(*f, CompletionKind::Function))
        .collect()
}

/// Keywords that come after a table name
fn sql_clause_keywords() -> Vec<CompletionItem> {
    let keywords = [
        "WHERE", "ORDER BY", "GROUP BY", "HAVING", "INNER JOIN", "LEFT JOIN",
        "RIGHT JOIN", "FULL JOIN", "CROSS JOIN", "ON", "AS", "WITH",
    ];
    
    keywords
        .iter()
        .map(|kw| CompletionItem::new(*kw, CompletionKind::Keyword))
        .collect()
}
