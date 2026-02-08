//! Application actions - async operations and business logic
//!
//! This module contains the core actions that modify application state,
//! including query execution, schema loading, and other async operations.

use crate::app::{App, ActivePanel, InputMode, SchemaNode, SchemaNodeType, ColumnCache};
use crate::sql::format_sql_query;
use crate::db::SchemaExplorer;
use anyhow::Result;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::oneshot;
use tokio::sync::Mutex;
use rust_i18n::t;

impl App {
    /// Execute the default query on startup
    pub async fn execute_default_query(&mut self) {
        if self.query.is_empty() || !self.is_connected() {
            return;
        }

        let client_arc = self.db().client();
        let mut client = client_arc.lock().await;
        let database = self.db().config.database.clone();

        match crate::db::QueryExecutor::execute(&mut client, &self.query).await {
            Ok(result) => {
                let row_count = result.row_count;
                let exec_time = result.execution_time.as_millis() as u64;

                self.history.add(
                    self.query.clone(),
                    exec_time,
                    Some(row_count),
                    database,
                );

                self.message = Some(t!(
                    "rows_returned",
                    count = row_count,
                    time = format!("{:.2}", result.execution_time.as_secs_f64() * 1000.0)
                ).to_string());

                self.result = result;
                self.results_selected = 0;
                self.results_col_selected = 0;
                self.results_col_scroll = 0;
            }
            Err(e) => {
                self.error = Some(e.to_string());
            }
        }
    }

    /// Load schema tree from database
    pub async fn load_schema(&mut self) -> Result<()> {
        if !self.is_connected() {
            return Ok(());
        }

        let client_arc = self.db().client();
        let mut client = client_arc.lock().await;

        // Create root folders
        let mut tables_folder = SchemaNode::new_folder("Tables");
        let mut views_folder = SchemaNode::new_folder("Views");
        let mut procs_folder = SchemaNode::new_folder("Stored Procedures");

        // Helper to get or create schema subfolder
        fn get_or_create_schema_folder<'a>(
            parent: &'a mut SchemaNode,
            schema_folders: &mut HashMap<String, usize>,
            schema_name: &str,
        ) -> &'a mut SchemaNode {
            if let Some(&idx) = schema_folders.get(schema_name) {
                &mut parent.children[idx]
            } else {
                let idx = parent.children.len();
                let mut folder = SchemaNode::new_folder(schema_name);
                folder.schema = Some(schema_name.to_string());
                parent.children.push(folder);
                schema_folders.insert(schema_name.to_string(), idx);
                &mut parent.children[idx]
            }
        }

        // Load tables grouped by schema
        let mut table_schema_folders: HashMap<String, usize> = HashMap::new();
        if let Ok(tables) = crate::db::SchemaExplorer::get_tables(&mut client, None).await {
            for table in tables {
                let schema_folder = get_or_create_schema_folder(
                    &mut tables_folder,
                    &mut table_schema_folders,
                    &table.schema,
                );
                schema_folder.children.push(SchemaNode {
                    name: table.name.clone(),
                    node_type: SchemaNodeType::Table,
                    expanded: false,
                    children: Vec::new(),
                    schema: Some(table.schema),
                });
            }
        }

        // Load views grouped by schema
        let mut view_schema_folders: HashMap<String, usize> = HashMap::new();
        if let Ok(views) = crate::db::SchemaExplorer::get_views(&mut client, None).await {
            for view in views {
                let schema_folder = get_or_create_schema_folder(
                    &mut views_folder,
                    &mut view_schema_folders,
                    &view.schema,
                );
                schema_folder.children.push(SchemaNode {
                    name: view.name.clone(),
                    node_type: SchemaNodeType::View,
                    expanded: false,
                    children: Vec::new(),
                    schema: Some(view.schema),
                });
            }
        }

        // Load procedures grouped by schema
        let mut proc_schema_folders: HashMap<String, usize> = HashMap::new();
        if let Ok(procs) = crate::db::SchemaExplorer::get_procedures(&mut client, None).await {
            for proc in procs {
                let schema_folder = get_or_create_schema_folder(
                    &mut procs_folder,
                    &mut proc_schema_folders,
                    &proc.schema,
                );
                schema_folder.children.push(SchemaNode {
                    name: proc.name.clone(),
                    node_type: SchemaNodeType::Procedure,
                    expanded: false,
                    children: Vec::new(),
                    schema: Some(proc.schema),
                });
            }
        }

        self.schema_tree = vec![tables_folder, views_folder, procs_folder];

        Ok(())
    }

    /// Start loading columns in background for autocomplete
    /// This spawns a background task that loads columns for all tables/views
    pub fn start_column_loading(&mut self) {
        if !self.is_connected() || self.columns_loading {
            return;
        }

        self.columns_loading = true;

        // Collect all tables and views from schema_tree
        let mut tables_to_load: Vec<(String, String)> = Vec::new();
        
        for root_folder in &self.schema_tree {
            // Only load columns for Tables and Views folders
            if root_folder.name != "Tables" && root_folder.name != "Views" {
                continue;
            }
            
            for schema_folder in &root_folder.children {
                let schema_name = &schema_folder.name;
                for obj in &schema_folder.children {
                    if obj.node_type == SchemaNodeType::Table || obj.node_type == SchemaNodeType::View {
                        tables_to_load.push((schema_name.clone(), obj.name.clone()));
                    }
                }
            }
        }

        let client_arc = self.db().client();
        let column_cache = Arc::clone(&self.column_cache);

        // Spawn background task to load all columns
        tokio::spawn(async move {
            Self::load_columns_background(client_arc, column_cache, tables_to_load).await;
        });
    }

    /// Background task to load columns for all tables/views
    async fn load_columns_background(
        client: Arc<Mutex<tiberius::Client<tokio_util::compat::Compat<tokio::net::TcpStream>>>>,
        column_cache: ColumnCache,
        tables: Vec<(String, String)>,
    ) {
        for (schema, table) in tables {
            // Lock client, load columns, release lock quickly
            let columns = {
                let mut client_guard = client.lock().await;
                SchemaExplorer::get_columns(&mut client_guard, &schema, &table).await
            };

            if let Ok(cols) = columns {
                // Insert into cache
                let mut cache = column_cache.write().await;
                cache.insert((schema, table), cols);
            }
            
            // Small yield to not block the event loop
            tokio::task::yield_now().await;
        }
    }

    /// Start query execution (non-blocking)
    pub fn start_query(&mut self) {
        if !self.is_connected() {
            self.error = Some("Não conectado ao banco de dados".to_string());
            return;
        }

        let query_text = if self.input_mode == InputMode::Visual {
            self.get_selected_text()
        } else {
            self.query.clone()
        };

        if query_text.trim().is_empty() || self.is_loading {
            return;
        }

        self.is_loading = true;
        self.error = None;
        self.message = None;
        self.spinner_frame = 0;

        let (tx, rx) = oneshot::channel();
        let client_arc = self.db().client();

        self.pending_query = Some(rx);
        self.pending_query_text = Some(query_text.clone());

        // Spawn query execution in background
        tokio::spawn(async move {
            let mut client = client_arc.lock().await;
            let result = crate::db::QueryExecutor::execute(&mut client, &query_text).await;

            let _ = tx.send(match result {
                Ok(r) => Ok(r),
                Err(e) => {
                    let mut error_msg = e.to_string();
                    let mut source = e.source();
                    while let Some(s) = source {
                        error_msg.push_str(&format!(" | Caused by: {}", s));
                        source = std::error::Error::source(s);
                    }
                    Err(error_msg)
                }
            });
        });
    }

    /// Check if query execution is complete and process result
    pub fn check_query_completion(&mut self) {
        if let Some(ref mut rx) = self.pending_query {
            match rx.try_recv() {
                Ok(result) => {
                    match result {
                        Ok(query_result) => {
                            let row_count = query_result.row_count;
                            let exec_time = query_result.execution_time.as_millis() as u64;

                            if let Some(ref query_text) = self.pending_query_text {
                                let database = self.db
                                    .as_ref()
                                    .map(|d| d.config.database.clone())
                                    .unwrap_or_default();
                                self.history.add(
                                    query_text.clone(),
                                    exec_time,
                                    Some(row_count),
                                    database,
                                );
                            }

                            self.message = Some(t!(
                                "rows_returned",
                                count = row_count,
                                time = format!("{:.2}", query_result.execution_time.as_secs_f64() * 1000.0)
                            ).to_string());

                            self.result = query_result;
                            self.results_scroll = 0;
                            self.results_selected = 0;
                        }
                        Err(error_msg) => {
                            self.error = Some(error_msg);
                        }
                    }

                    self.is_loading = false;
                    self.pending_query = None;
                    self.pending_query_text = None;
                }
                Err(oneshot::error::TryRecvError::Empty) => {
                    // Still waiting
                }
                Err(oneshot::error::TryRecvError::Closed) => {
                    // Channel closed unexpectedly
                    self.error = Some(t!("query_interrupted").to_string());
                    self.is_loading = false;
                    self.pending_query = None;
                    self.pending_query_text = None;
                }
            }
        }
    }

    /// Toggle schema node expansion
    pub fn toggle_schema_node(&mut self) {
        let mut current_idx = 0;
        let path = Self::find_node_path(&self.schema_tree, self.schema_selected, &mut current_idx);
        
        if let Some(path) = path {
            Self::toggle_node_at_path(&mut self.schema_tree, &path);
        }
    }

    /// Find the path (indices) to reach the node at the given visible index
    fn find_node_path(nodes: &[SchemaNode], target_idx: usize, current_idx: &mut usize) -> Option<Vec<usize>> {
        for (i, node) in nodes.iter().enumerate() {
            if *current_idx == target_idx {
                return Some(vec![i]);
            }
            *current_idx += 1;
            
            if node.expanded {
                if let Some(mut child_path) = Self::find_node_path(&node.children, target_idx, current_idx) {
                    let mut path = vec![i];
                    path.append(&mut child_path);
                    return Some(path);
                }
            }
        }
        None
    }

    /// Toggle the node at the given path
    fn toggle_node_at_path(nodes: &mut [SchemaNode], path: &[usize]) {
        if path.is_empty() {
            return;
        }
        
        if path.len() == 1 {
            if let Some(node) = nodes.get_mut(path[0]) {
                node.expanded = !node.expanded;
            }
        } else if let Some(node) = nodes.get_mut(path[0]) {
            Self::toggle_node_at_path(&mut node.children, &path[1..]);
        }
    }

    /// Insert selected table/view into query
    pub async fn insert_schema_object(&mut self) {
        if !self.is_connected() {
            return;
        }

        let client_arc = self.db().client();
        let mut client = client_arc.lock().await;
        let visible = self.get_visible_schema_nodes();
        if let Some((_, node)) = visible.get(self.schema_selected) {
            if node.node_type == SchemaNodeType::Table || node.node_type == SchemaNodeType::View {
                let full_name = if let Some(ref schema) = node.schema {
                    format!("{}.{}", schema, node.name)
                } else {
                    node.name.clone()
                };
                let insert_text = format!("[{}]", full_name);
                self.query.insert_str(self.cursor_pos, &insert_text);
                self.cursor_pos += insert_text.len();
                self.active_panel = ActivePanel::QueryEditor;
            }
            else if node.node_type == SchemaNodeType::Procedure {
                let schema = node.schema.clone().unwrap_or_else(|| "dbo".to_string());
                let name = node.name.clone();
                if let Ok(definition) = SchemaExplorer::get_procedure_definition(&mut client, schema, name).await {
                    self.save_undo_state();
                    self.query = definition;
                    self.cursor_pos = 0;
                    self.active_panel = ActivePanel::QueryEditor;
                }
            }
        }
    }

    /// Load history entry into query
    pub fn load_history_entry(&mut self) {
        let entries = self.history.entries();
        let entry_query = entries
            .get(entries.len().saturating_sub(1).saturating_sub(self.history_selected))
            .map(|e| e.query.clone());
        
        if let Some(query) = entry_query {
            self.save_undo_state();
            self.query = query;
            self.cursor_pos = self.query.len();
            self.active_panel = ActivePanel::QueryEditor;
        }
    }

    /// Format SQL query with proper indentation and line breaks
    pub fn format_sql(&mut self) {
        self.save_undo_state();
        let formatted = format_sql_query(&self.query);
        self.query = formatted;
        self.cursor_pos = self.query.len();
        self.query_scroll_x = 0;
        self.query_scroll_y = 0;
    }

    /// Delete selected text in visual mode
    pub fn delete_selection(&mut self) {
        let (start, end) = self.get_visual_selection();
        let end_inclusive = (end + 1).min(self.query.len());
        self.query.drain(start..end_inclusive);
        self.cursor_pos = start.min(self.query.len().saturating_sub(1));
        self.input_mode = InputMode::Normal;
    }

    /// Yank (copy) selected text to clipboard
    pub fn yank_selection(&mut self) -> Option<String> {
        let text = self.get_selected_text();
        self.input_mode = InputMode::Normal;
        Some(text)
    }
}
