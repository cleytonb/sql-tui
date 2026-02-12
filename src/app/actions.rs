//! Application actions - async operations and business logic
//!
//! This module contains the core actions that modify application state,
//! including query execution, schema loading, and other async operations.

use crate::app::{App, ActivePanel, InputMode, SchemaNode, SchemaNodeType, ColumnCache};
use crate::db::{DatabaseBackend, DatabaseDriver, ColumnDef};
use crate::sql::format_sql_query;
use anyhow::Result;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::oneshot;
use rust_i18n::t;

impl App {
    /// Load schema tree from database
    pub async fn load_schema(&mut self) -> Result<()> {
        if !self.is_connected() {
            return Ok(());
        }

        let db = self.db.as_ref().unwrap();

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
        if let Ok(tables) = db.get_tables(None).await {
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
        if let Ok(views) = db.get_views(None).await {
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
        if let Ok(procs) = db.get_procedures(None).await {
            for proc_obj in procs {
                let schema_folder = get_or_create_schema_folder(
                    &mut procs_folder,
                    &mut proc_schema_folders,
                    &proc_obj.schema,
                );
                schema_folder.children.push(SchemaNode {
                    name: proc_obj.name.clone(),
                    node_type: SchemaNodeType::Procedure,
                    expanded: false,
                    children: Vec::new(),
                    schema: Some(proc_obj.schema),
                });
            }
        }

        self.schema_tree = vec![tables_folder, views_folder, procs_folder];

        Ok(())
    }

    /// Start loading columns in background for autocomplete
    pub fn start_column_loading(&mut self) {
        if !self.is_connected() || self.columns_loading {
            return;
        }

        self.columns_loading = true;

        // Collect all tables and views from schema_tree
        let mut tables_to_load: Vec<(String, String)> = Vec::new();

        for root_folder in &self.schema_tree {
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

        let column_cache = Arc::clone(&self.column_cache);

        // We need to dispatch to the correct driver-specific background loading.
        // Since the trait is behind Box<dyn>, we downcast or use driver-specific paths.
        let db = self.db.as_ref().unwrap();

        match db.backend() {
            DatabaseBackend::SqlServer => {
                // For SQL Server, we can grab the Arc<Mutex<Client>> for background use
                // We need to downcast to SqlServerDriver
                let db_ptr = self.db.as_ref().unwrap();
                // SAFETY: We just checked backend() == SqlServer
                let sqlserver: &crate::db::sqlserver::SqlServerDriver =
                    unsafe { &*(db_ptr.as_ref() as *const dyn DatabaseDriver as *const crate::db::sqlserver::SqlServerDriver) };
                let client_arc = sqlserver.client_arc();

                tokio::spawn(async move {
                    Self::load_columns_background_sqlserver(client_arc, column_cache, tables_to_load).await;
                });
            }
            DatabaseBackend::Sqlite => {
                // For SQLite, we run column loading synchronously in a spawn_blocking
                // We need the path to re-open a connection for the background task
                let db_ptr = self.db.as_ref().unwrap();
                let sqlite: &crate::db::sqlite::SqliteDriver =
                    unsafe { &*(db_ptr.as_ref() as *const dyn DatabaseDriver as *const crate::db::sqlite::SqliteDriver) };
                let path = sqlite.path.clone();

                tokio::spawn(async move {
                    Self::load_columns_background_sqlite(path, column_cache, tables_to_load).await;
                });
            }
        }
    }

    /// Background column loading for SQL Server
    async fn load_columns_background_sqlserver(
        client: Arc<tokio::sync::Mutex<tiberius::Client<tokio_util::compat::Compat<tokio::net::TcpStream>>>>,
        column_cache: ColumnCache,
        tables: Vec<(String, String)>,
    ) {
        for (schema, table) in tables {
            let columns = {
                let mut client_guard = client.lock().await;
                // Re-use the SchemaExplorer-style query inline
                let query = format!(
                    "SELECT c.name, t.name, c.is_nullable, \
                     ISNULL(pk.is_primary_key, 0), c.is_identity, \
                     c.max_length, c.precision, c.scale \
                     FROM sys.columns c \
                     INNER JOIN sys.types t ON c.user_type_id = t.user_type_id \
                     INNER JOIN sys.tables tbl ON c.object_id = tbl.object_id \
                     INNER JOIN sys.schemas s ON tbl.schema_id = s.schema_id \
                     LEFT JOIN ( \
                        SELECT ic.column_id, ic.object_id, 1 as is_primary_key \
                        FROM sys.index_columns ic \
                        INNER JOIN sys.indexes i ON ic.object_id = i.object_id AND ic.index_id = i.index_id \
                        WHERE i.is_primary_key = 1 \
                     ) pk ON c.object_id = pk.object_id AND c.column_id = pk.column_id \
                     WHERE s.name = '{}' AND tbl.name = '{}' \
                     ORDER BY c.column_id",
                    schema, table
                );

                let result = client_guard.simple_query(&query).await;
                match result {
                    Ok(stream) => {
                        let results = stream.into_results().await;
                        match results {
                            Ok(results) => {
                                let mut cols = Vec::new();
                                for result in results {
                                    for row in result {
                                        cols.push(ColumnDef {
                                            name: row.get::<&str, _>(0).unwrap_or("").to_string(),
                                            data_type: row.get::<&str, _>(1).unwrap_or("").to_string(),
                                            is_nullable: row.get::<bool, _>(2).unwrap_or(true),
                                            is_primary_key: row.get::<i32, _>(3).unwrap_or(0) == 1,
                                            is_identity: row.get::<bool, _>(4).unwrap_or(false),
                                            max_length: row.get::<i16, _>(5).map(|v| v as i32),
                                            precision: row.get::<u8, _>(6).map(|v| v as i32),
                                            scale: row.get::<u8, _>(7).map(|v| v as i32),
                                        });
                                    }
                                }
                                Ok(cols)
                            }
                            Err(e) => Err(e.into()),
                        }
                    }
                    Err(e) => Err::<Vec<ColumnDef>, anyhow::Error>(e.into()),
                }
            };

            if let Ok(cols) = columns {
                let mut cache = column_cache.write().await;
                cache.insert((schema, table), cols);
            }

            tokio::task::yield_now().await;
        }
    }

    /// Background column loading for SQLite
    async fn load_columns_background_sqlite(
        path: std::path::PathBuf,
        column_cache: ColumnCache,
        tables: Vec<(String, String)>,
    ) {
        let p = path.clone();
        let result = tokio::task::spawn_blocking(move || -> Vec<(String, String, Vec<ColumnDef>)> {
            let conn = match rusqlite::Connection::open(&p) {
                Ok(c) => c,
                Err(_) => return Vec::new(),
            };
            let mut results = Vec::new();
            for (schema, table) in &tables {
                let query = format!("PRAGMA table_info('{}')", table);
                let mut stmt = match conn.prepare(&query) {
                    Ok(s) => s,
                    Err(_) => continue,
                };
                let mut cols = Vec::new();
                let mut rows = match stmt.query([]) {
                    Ok(r) => r,
                    Err(_) => continue,
                };
                while let Ok(Some(row)) = rows.next() {
                    let name: String = row.get(1).unwrap_or_default();
                    let data_type: String = row.get(2).unwrap_or_default();
                    let not_null: bool = row.get(3).unwrap_or(false);
                    let pk: i32 = row.get(5).unwrap_or(0);
                    cols.push(ColumnDef {
                        name,
                        data_type,
                        is_nullable: !not_null,
                        is_primary_key: pk > 0,
                        is_identity: false,
                        max_length: None,
                        precision: None,
                        scale: None,
                    });
                }
                results.push((schema.clone(), table.clone(), cols));
            }
            results
        })
        .await;

        if let Ok(entries) = result {
            let mut cache = column_cache.write().await;
            for (schema, table, cols) in entries {
                cache.insert((schema, table), cols);
            }
        }
    }

    /// Start query execution (non-blocking)
    pub fn start_query(&mut self) {
        if !self.is_connected() {
            self.error = Some(t!("not_connected_to_database").to_string());
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

        self.pending_query = Some(rx);
        self.pending_query_text = Some(query_text.clone());

        let db = self.db.as_ref().unwrap();

        match db.backend() {
            DatabaseBackend::SqlServer => {
                let sqlserver: &crate::db::sqlserver::SqlServerDriver =
                    unsafe { &*(db.as_ref() as *const dyn DatabaseDriver as *const crate::db::sqlserver::SqlServerDriver) };
                let client_arc = sqlserver.client_arc();

                tokio::spawn(async move {
                    let mut client = client_arc.lock().await;
                    let result = crate::db::sqlserver::SqlServerDriver::execute_query_with_client(&mut client, &query_text).await;

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
            DatabaseBackend::Sqlite => {
                let sqlite: &crate::db::sqlite::SqliteDriver =
                    unsafe { &*(db.as_ref() as *const dyn DatabaseDriver as *const crate::db::sqlite::SqliteDriver) };
                let path = sqlite.path.clone();

                tokio::spawn(async move {
                    // Open a new connection for the background query
                    let result = async {
                        let driver = crate::db::sqlite::SqliteDriver::new(path).await?;
                        driver.execute_query(&query_text).await
                    }.await;

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
        }
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
                                    .map(|d| d.database_name())
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

        let visible = self.get_visible_schema_nodes();
        if let Some((_, node)) = visible.get(self.schema_selected) {
            if node.node_type == SchemaNodeType::Table || node.node_type == SchemaNodeType::View {
                let full_name = if let Some(ref schema) = node.schema {
                    format!("{}.{}", schema, node.name)
                } else {
                    node.name.clone()
                };
                let insert_text = format!("[{}]", full_name);
                self.query.insert_str(self.query_byte_pos(), &insert_text);
                self.cursor_pos += insert_text.chars().count();
                self.active_panel = ActivePanel::QueryEditor;
            } else if node.node_type == SchemaNodeType::Procedure {
                let schema = node.schema.clone().unwrap_or_else(|| "dbo".to_string());
                let name = node.name.clone();
                let db = self.db.as_ref().unwrap();
                if let Ok(definition) = db.get_procedure_definition(&schema, &name).await {
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
            self.cursor_pos = self.query.chars().count();
            self.active_panel = ActivePanel::QueryEditor;
        }
    }

    /// Format SQL query with proper indentation and line breaks
    pub fn format_sql(&mut self) {
        self.save_undo_state();
        let formatted = format_sql_query(&self.query);
        self.query = formatted;
        self.cursor_pos = self.query.chars().count();
        self.query_scroll_x = 0;
        self.query_scroll_y = 0;
    }

    /// Delete selected text in visual mode
    pub fn delete_selection(&mut self) {
        let (start, end) = self.get_visual_selection();
        let char_count = self.query.chars().count();
        let end_inclusive = (end + 1).min(char_count);
        let byte_start = Self::char_to_byte_index(&self.query, start);
        let byte_end = Self::char_to_byte_index(&self.query, end_inclusive);
        self.query.drain(byte_start..byte_end);
        self.cursor_pos = start.min(self.query.chars().count().saturating_sub(1));
        self.input_mode = InputMode::Normal;
    }

    /// Yank (copy) selected text to clipboard
    pub fn yank_selection(&mut self) -> Option<String> {
        let text = self.get_selected_text();
        self.input_mode = InputMode::Normal;
        Some(text)
    }
}
