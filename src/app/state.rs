//! Application state

use crate::db::{DbConfig, DbConnection, QueryResult};
use crate::app::QueryHistory;
use anyhow::Result;
use std::error::Error;
use tokio::sync::oneshot;

/// Active panel in the UI
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ActivePanel {
    QueryEditor,
    Results,
    SchemaExplorer,
    History,
}

/// Results tab view
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ResultsTab {
    Data,       // Table data
    Columns,    // Column names and types
    Stats,      // Query statistics
}

/// Input mode for the query editor
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum InputMode {
    Normal,
    Insert,
    Command,
    Visual
}

/// Schema tree node
#[derive(Clone, Debug)]
pub struct SchemaNode {
    pub name: String,
    pub node_type: SchemaNodeType,
    pub expanded: bool,
    pub children: Vec<SchemaNode>,
    pub schema: Option<String>,
}

#[derive(Clone, Debug, PartialEq)]
pub enum SchemaNodeType {
    Database,
    Folder,
    Table,
    View,
    Procedure,
    Function,
    Column,
}

impl SchemaNode {
    pub fn new_folder(name: &str) -> Self {
        Self {
            name: name.to_string(),
            node_type: SchemaNodeType::Folder,
            expanded: false,
            children: Vec::new(),
            schema: None,
        }
    }

    pub fn icon(&self) -> &'static str {
        match self.node_type {
            SchemaNodeType::Database => "🗄️ ",
            SchemaNodeType::Folder => if self.expanded { "📂" } else { "📁" },
            SchemaNodeType::Table => "📋",
            SchemaNodeType::View => "👁️ ",
            SchemaNodeType::Procedure => "⚙️ ",
            SchemaNodeType::Function => "ƒ ",
            SchemaNodeType::Column => "├─",
        }
    }
}

/// Main application state
pub struct App {
    /// Database connection
    pub db: DbConnection,

    /// Current query text
    pub query: String,

    /// Cursor position in query
    pub cursor_pos: usize,

    /// Current query result
    pub result: QueryResult,

    /// Is query running?
    pub is_loading: bool,

    /// Error message
    pub error: Option<String>,

    /// Success message
    pub message: Option<String>,

    /// Active panel
    pub active_panel: ActivePanel,

    /// Input mode (vim-style)
    pub input_mode: InputMode,

    /// Query history
    pub history: QueryHistory,

    /// Schema tree
    pub schema_tree: Vec<SchemaNode>,

    /// Selected index in schema tree
    pub schema_selected: usize,

    /// Results scroll position
    pub results_scroll: usize,

    /// Selected row in results
    pub results_selected: usize,

    /// Selected column in results
    pub results_col_selected: usize,

    /// Horizontal scroll offset for results columns
    pub results_col_scroll: usize,

    /// Number of columns that fit on screen (updated by UI)
    pub results_cols_visible: usize,

    /// Current results tab
    pub results_tab: ResultsTab,

    /// History scroll position
    pub history_selected: usize,

    /// Command buffer (for : commands)
    pub command_buffer: String,

    /// Should quit?
    pub should_quit: bool,

    /// Show help popup
    pub show_help: bool,

    /// Status message
    pub status: String,

    /// Server version
    pub server_version: String,

    /// Spinner frame for loading animation
    pub spinner_frame: usize,

    /// Pending query result receiver
    pub pending_query: Option<oneshot::Receiver<Result<QueryResult, String>>>,

    /// Query being executed (for history)
    pub pending_query_text: Option<String>,

    /// Query editor horizontal scroll offset
    pub query_scroll_x: usize,

    /// Query editor vertical scroll offset
    pub query_scroll_y: usize,

    /// Show search input in schema explorer
    pub show_search_schema: bool,

    /// Search query for schema explorer
    pub schema_search_query: String,

    /// Pending smooth scroll amount (positive = down, negative = up)
    pub pending_scroll: i32,
}

/// Spinner animation frames
pub const SPINNER_FRAMES: &[&str] = &["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"];

impl App {
    /// Create new app with database connection
    pub async fn new() -> Result<Self> {
        let config = DbConfig::default();
        let db = DbConnection::new(config).await?;

        let server_version = db.get_server_version().await.unwrap_or_else(|_| "Unknown".to_string());
        let short_version = server_version.lines().next().unwrap_or("SQL Server").to_string();

        // Default query for quick testing
        let default_query = "SELECT TOP 2 * FROM pmt.Contas".to_string();
        let cursor_pos = default_query.len();

        let mut app = Self {
            db,
            query: default_query,
            cursor_pos,
            result: QueryResult::empty(),
            is_loading: false,
            error: None,
            message: Some("Connected to SQL Server".to_string()),
            active_panel: ActivePanel::QueryEditor,
            input_mode: InputMode::Insert,
            history: QueryHistory::new(1000),
            schema_tree: Vec::new(),
            schema_selected: 0,
            results_scroll: 0,
            results_selected: 0,
            results_col_selected: 0,
            results_col_scroll: 0,
            results_cols_visible: 5, // default, será atualizado pelo UI
            results_tab: ResultsTab::Data,
            history_selected: 0,
            command_buffer: String::new(),
            should_quit: false,
            show_help: false,
            status: format!("Connected | {}", short_version),
            server_version: short_version,
            spinner_frame: 0,
            pending_query: None,
            pending_query_text: None,
            query_scroll_x: 0,
            query_scroll_y: 0,
            show_search_schema: false,
            schema_search_query: String::new(),
            pending_scroll: 0,
        };

        // Load initial schema
        app.load_schema().await?;

        // Auto-execute default query to show results on startup
        app.execute_default_query().await;

        Ok(app)
    }

    /// Execute the default query on startup
    async fn execute_default_query(&mut self) {
        if self.query.is_empty() {
            return;
        }

        let client_arc = self.db.client();
        let mut client = client_arc.lock().await;

        match crate::db::QueryExecutor::execute(&mut client, &self.query).await {
            Ok(result) => {
                let row_count = result.row_count;
                let exec_time = result.execution_time.as_millis() as u64;

                self.history.add(
                    self.query.clone(),
                    exec_time,
                    Some(row_count),
                    self.db.config.database.clone(),
                );

                self.message = Some(format!(
                    "{} row(s) returned in {:.2}ms",
                    row_count,
                    result.execution_time.as_secs_f64() * 1000.0
                ));

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

    /// Load schema tree
    pub async fn load_schema(&mut self) -> Result<()> {
        use std::collections::HashMap;

        let client_arc = self.db.client();
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

    /// Start query execution (non-blocking)
    pub fn start_query(&mut self) {
        if self.query.trim().is_empty() || self.is_loading {
            return;
        }

        self.is_loading = true;
        self.error = None;
        self.message = None;
        self.spinner_frame = 0;

        let (tx, rx) = oneshot::channel();
        let client_arc = self.db.client();
        let query = self.query.clone();

        self.pending_query = Some(rx);
        self.pending_query_text = Some(query.clone());

        // Spawn query execution in background
        tokio::spawn(async move {
            let mut client = client_arc.lock().await;
            let result = crate::db::QueryExecutor::execute(&mut client, &query).await;

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
                                self.history.add(
                                    query_text.clone(),
                                    exec_time,
                                    Some(row_count),
                                    self.db.config.database.clone(),
                                );
                            }

                            self.message = Some(format!(
                                "{} row(s) returned in {:.2}ms",
                                row_count,
                                query_result.execution_time.as_secs_f64() * 1000.0
                            ));

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
                    self.error = Some("Query execution was interrupted".to_string());
                    self.is_loading = false;
                    self.pending_query = None;
                    self.pending_query_text = None;
                }
            }
        }
    }

    /// Get flattened schema tree for display
    pub fn get_visible_schema_nodes(&self) -> Vec<(usize, &SchemaNode)> {
        let mut nodes = Vec::new();
        
        // Se há busca ativa, filtra os nós
        if !self.schema_search_query.is_empty() {
            let query = self.schema_search_query.to_lowercase();
            for node in &self.schema_tree {
                Self::flatten_node_filtered(node, 0, &mut nodes, &query);
            }
        } else {
            for node in &self.schema_tree {
                Self::flatten_node(node, 0, &mut nodes);
            }
        }
        nodes
    }

    fn flatten_node<'a>(node: &'a SchemaNode, depth: usize, nodes: &mut Vec<(usize, &'a SchemaNode)>) {
        nodes.push((depth, node));
        if node.expanded {
            for child in &node.children {
                Self::flatten_node(child, depth + 1, nodes);
            }
        }
    }

    /// Flatten node with filter - shows matching nodes and their parents
    fn flatten_node_filtered<'a>(
        node: &'a SchemaNode,
        depth: usize,
        nodes: &mut Vec<(usize, &'a SchemaNode)>,
        query: &str,
    ) {
        let node_matches = node.name.to_lowercase().contains(query);
        let has_matching_children = Self::has_matching_children(node, query);

        // Mostra o nó se ele ou algum filho corresponde à busca
        if node_matches || has_matching_children {
            nodes.push((depth, node));

            // Se tem filhos que correspondem, mostra todos os filhos recursivamente
            for child in &node.children {
                Self::flatten_node_filtered(child, depth + 1, nodes, query);
            }
        }
    }

    /// Check if a node or any of its descendants match the query
    fn has_matching_children(node: &SchemaNode, query: &str) -> bool {
        for child in &node.children {
            if child.name.to_lowercase().contains(query) {
                return true;
            }
            if Self::has_matching_children(child, query) {
                return true;
            }
        }
        false
    }

    /// Toggle schema node expansion
    pub fn toggle_schema_node(&mut self) {
        // Build path to selected node by tracking indices
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
    pub fn insert_schema_object(&mut self) {
        let visible = self.get_visible_schema_nodes();
        if let Some((_, node)) = visible.get(self.schema_selected) {
            if node.node_type == SchemaNodeType::Table || node.node_type == SchemaNodeType::View {
                // Build full name with schema if available
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
        }
    }

    /// Load history entry into query
    pub fn load_history_entry(&mut self) {
        let entries = self.history.entries();
        if let Some(entry) = entries.get(entries.len().saturating_sub(1).saturating_sub(self.history_selected)) {
            self.query = entry.query.clone();
            self.cursor_pos = self.query.len();
            self.active_panel = ActivePanel::QueryEditor;
        }
    }

    /// Update scroll position to keep cursor visible
    pub fn update_scroll(&mut self, visible_width: usize, visible_height: usize) {
        // Calculate current line and column
        let (line, col) = self.get_cursor_line_col();

        // Horizontal scroll - keep cursor visible with some margin
        let margin = 5;
        if col < self.query_scroll_x {
            self.query_scroll_x = col.saturating_sub(margin);
        } else if col >= self.query_scroll_x + visible_width.saturating_sub(margin) {
            self.query_scroll_x = col.saturating_sub(visible_width.saturating_sub(margin + 1));
        }

        // Vertical scroll - keep cursor visible
        if line < self.query_scroll_y {
            self.query_scroll_y = line;
        } else if line >= self.query_scroll_y + visible_height {
            self.query_scroll_y = line.saturating_sub(visible_height.saturating_sub(1));
        }
    }

    /// Get cursor line and column
    pub fn get_cursor_line_col(&self) -> (usize, usize) {
        let mut line = 0;
        let mut col = 0;

        for (i, ch) in self.query.chars().enumerate() {
            if i >= self.cursor_pos {
                break;
            }
            if ch == '\n' {
                line += 1;
                col = 0;
            } else {
                col += 1;
            }
        }

        (line, col)
    }

    /// Format SQL query with proper indentation and line breaks
    pub fn format_sql(&mut self) {
        let formatted = format_sql_query(&self.query);
        self.query = formatted;
        self.cursor_pos = self.query.len();
        self.query_scroll_x = 0;
        self.query_scroll_y = 0;
    }
}

/// SQL formatter - formats SQL with proper indentation and line breaks
fn format_sql_query(sql: &str) -> String {
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
