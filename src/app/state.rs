//! Application state - core data structures and state management
//!
//! This module contains the main App struct and related types.
//! Business logic and async operations are in the actions module.

use crate::completion::CompletionState;
use crate::config::{AppConfig, ConnectionConfig, ConnectionForm};
use crate::db::{ColumnDef, DbConnection, QueryResult};
use crate::app::{QueryHistory, UndoManager};
use anyhow::Result;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{oneshot, RwLock};
use rust_i18n::t;

/// Cache key for columns: (schema, table_or_view)
pub type ColumnCacheKey = (String, String);

/// Thread-safe column cache
pub type ColumnCache = Arc<RwLock<HashMap<ColumnCacheKey, Vec<ColumnDef>>>>;

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

/// Spinner animation frames
pub const SPINNER_FRAMES: &[&str] = &["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"];

/// Which panel is focused in the connection modal
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ConnectionModalFocus {
    /// Left panel - list of connections
    List,
    /// Right panel - connection form
    Form,
}

/// Main application state
pub struct App {
    // === Database ===
    /// Database connection (None when not connected yet)
    pub db: Option<DbConnection>,

    // === Connection Management ===
    /// Application configuration (saved connections)
    pub app_config: AppConfig,
    /// Show connection modal
    pub show_connection_modal: bool,
    /// Selected index in connection list (includes "+ Criar nova" at the end)
    pub connection_list_selected: usize,
    /// Connection form for editing/creating
    pub connection_form: ConnectionForm,
    /// Current focused field in the form (0-5)
    pub connection_form_focus: usize,
    /// Which panel is focused in the modal
    pub connection_modal_focus: ConnectionModalFocus,

    // === Query Editor ===
    /// Current query text
    pub query: String,
    /// Cursor position in query
    pub cursor_pos: usize,
    /// Query editor horizontal scroll offset
    pub query_scroll_x: usize,
    /// Query editor vertical scroll offset
    pub query_scroll_y: usize,
    /// Input mode (vim-style)
    pub input_mode: InputMode,
    /// Visual mode selection anchor (start position)
    pub visual_anchor: usize,
    /// Last character search for f/F/t/T with ; and , repeat
    /// (character, is_forward, is_till)
    pub last_char_search: Option<(char, bool, bool)>,
    /// Pending operator waiting for character input (f, F, t, T)
    pub pending_char_search: Option<char>,
    /// Pending g prefix (waiting for second key: g, _, e, etc.)
    pub pending_g: bool,
    /// Autocomplete state
    pub completion: CompletionState,

    // === Query Execution ===
    /// Current query result
    pub result: QueryResult,
    /// Is query running?
    pub is_loading: bool,
    /// Pending query result receiver
    pub pending_query: Option<oneshot::Receiver<Result<QueryResult, String>>>,
    /// Query being executed (for history)
    pub pending_query_text: Option<String>,
    /// Spinner frame for loading animation
    pub spinner_frame: usize,

    // === Results Panel ===
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

    // === Schema Explorer ===
    /// Schema tree
    pub schema_tree: Vec<SchemaNode>,
    /// Selected index in schema tree
    pub schema_selected: usize,
    /// Scroll offset for schema tree (for proper scroll behavior)
    pub schema_scroll_offset: usize,
    /// Show search input in schema explorer
    pub show_search_schema: bool,
    /// Search query for schema explorer
    pub schema_search_query: String,
    /// Column cache for autocomplete (loaded in background)
    pub column_cache: ColumnCache,
    /// Whether columns are being loaded in background
    pub columns_loading: bool,

    // === History ===
    /// Query history
    pub history: QueryHistory,
    /// History scroll position
    pub history_selected: usize,
    /// Scroll offset for history panel (for proper scroll behavior)
    pub history_scroll_offset: usize,

    // === Undo/Redo ===
    /// Undo manager for query editor
    pub undo_manager: UndoManager,

    // === UI State ===
    /// Active panel
    pub active_panel: ActivePanel,
    /// Should quit?
    pub should_quit: bool,
    /// Show help popup
    pub show_help: bool,
    /// Error message
    pub error: Option<String>,
    /// Success message
    pub message: Option<String>,
    /// Status message
    pub status: String,
    /// Server version
    pub server_version: String,
    /// Command buffer (for : commands)
    pub command_buffer: String,
    /// Pending smooth scroll amount (positive = down, negative = up)
    pub pending_scroll: i32,
    /// Command mode
    pub command_mode: bool,
}

impl App {
    /// Create new app, attempting to auto-connect to last used connection
    pub async fn new() -> Result<Self> {
        let app_config = AppConfig::load();
        
        // Initialize i18n locale from config or system
        crate::init_locale(app_config.locale.as_deref());
        
        // Try to connect to the last used connection
        let (db, show_modal, server_version) = if let Some(ref last_name) = app_config.last_connection {
            if let Some(conn_config) = app_config.get_connection(last_name) {
                match DbConnection::from_config(conn_config).await {
                    Ok(db) => {
                        let version = db.get_server_version().await
                            .unwrap_or_else(|_| "Unknown".to_string());
                        let short = version.lines().next().unwrap_or("SQL Server").to_string();
                        (Some(db), false, short)
                    }
                    Err(_) => (None, true, String::new()),
                }
            } else {
                (None, true, String::new())
            }
        } else {
            (None, true, String::new())
        };

        let is_connected = db.is_some();
        let cursor_pos = 0;

        let mut app = Self {
            db,
            app_config,
            show_connection_modal: show_modal,
            connection_list_selected: 0,
            connection_form: ConnectionForm::new_empty(),
            connection_form_focus: 0,
            connection_modal_focus: ConnectionModalFocus::List,
            query: String::new(), 
            cursor_pos,
            query_scroll_x: 0,
            query_scroll_y: 0,
            input_mode: InputMode::Insert,
            visual_anchor: 0,
            last_char_search: None,
            pending_char_search: None,
            pending_g: false,
            completion: CompletionState::new(),
            result: QueryResult::empty(),
            is_loading: false,
            pending_query: None,
            pending_query_text: None,
            spinner_frame: 0,
            results_scroll: 0,
            results_selected: 0,
            results_col_selected: 0,
            results_col_scroll: 0,
            results_cols_visible: 5,
            results_tab: ResultsTab::Data,
            schema_tree: Vec::new(),
            schema_selected: 0,
            schema_scroll_offset: 0,
            show_search_schema: false,
            schema_search_query: String::new(),
            column_cache: Arc::new(RwLock::new(HashMap::new())),
            columns_loading: false,
            history: QueryHistory::new(1000),
            history_selected: 0,
            history_scroll_offset: 0,
            undo_manager: UndoManager::new(1000),
            command_mode: false,
            active_panel: ActivePanel::QueryEditor,
            should_quit: false,
            show_help: false,
            error: None,
            message: if is_connected { Some(t!("connected").to_string()) } else { None },
            status: if is_connected { format!("{} | {}", t!("connected"), server_version) } else { t!("disconnected").to_string() },
            server_version,
            command_buffer: String::new(),
            pending_scroll: 0,
        };

        // If connected, load schema
        if is_connected {
            let _ = app.load_schema().await;
            // Start loading columns in background for autocomplete
            app.start_column_loading();
        }

        Ok(app)
    }

    /// Check if connected to a database
    pub fn is_connected(&self) -> bool {
        self.db.is_some()
    }

    /// Get database connection reference (panics if not connected)
    pub fn db(&self) -> &DbConnection {
        self.db.as_ref().expect("Not connected to database")
    }

    /// Get mutable database connection reference (panics if not connected)
    pub fn db_mut(&mut self) -> &mut DbConnection {
        self.db.as_mut().expect("Not connected to database")
    }

    /// Connect to a database using the given config
    pub async fn connect(&mut self, config: &ConnectionConfig) -> Result<()> {
        let db = DbConnection::from_config(config).await?;
        
        let version = db.get_server_version().await
            .unwrap_or_else(|_| "Unknown".to_string());
        let short_version = version.lines().next().unwrap_or("SQL Server").to_string();
        
        self.db = Some(db);
        self.server_version = short_version.clone();
        self.status = format!("Conectado | {}", short_version);
        self.message = Some(t!("connected_to", name = config.name).to_string());
        self.show_connection_modal = false;
        
        // Update last connection and save config
        self.app_config.set_last_connection(&config.name);
        let _ = self.app_config.save();
        
        // Load schema and start loading columns in background
        let _ = self.load_schema().await;
        self.start_column_loading();
        
        Ok(())
    }

    /// Get the number of items in the connection list (connections + "Criar nova")
    pub fn connection_list_len(&self) -> usize {
        self.app_config.connections.len() + 1  // +1 for "Criar nova"
    }

    /// Check if the selected item is "Criar nova"
    pub fn is_create_new_selected(&self) -> bool {
        self.connection_list_selected >= self.app_config.connections.len()
    }

    /// Get selected connection config (None if "Criar nova" is selected)
    pub fn get_selected_connection(&self) -> Option<&ConnectionConfig> {
        self.app_config.connections.get(self.connection_list_selected)
    }

    // === Query State Helpers ===

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

    /// Update scroll position to keep cursor visible
    pub fn update_scroll(&mut self, visible_width: usize, visible_height: usize) {
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

    /// Get visual selection range (start, end) - always start <= end
    pub fn get_visual_selection(&self) -> (usize, usize) {
        if self.visual_anchor <= self.cursor_pos {
            (self.visual_anchor, self.cursor_pos)
        } else {
            (self.cursor_pos, self.visual_anchor)
        }
    }

    /// Get selected text in visual mode
    pub fn get_selected_text(&self) -> String {
        let (start, end) = self.get_visual_selection();
        self.query.chars().skip(start).take(end - start + 1).collect()
    }

    // === Schema State Helpers ===

    /// Get flattened schema tree for display
    pub fn get_visible_schema_nodes(&self) -> Vec<(usize, &SchemaNode)> {
        let mut nodes = Vec::new();
        
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

    fn flatten_node_filtered<'a>(
        node: &'a SchemaNode,
        depth: usize,
        nodes: &mut Vec<(usize, &'a SchemaNode)>,
        query: &str,
    ) {
        let node_matches = node.name.to_lowercase().contains(query);
        let has_matching_children = Self::has_matching_children(node, query);

        if node_matches || has_matching_children {
            nodes.push((depth, node));
            for child in &node.children {
                Self::flatten_node_filtered(child, depth + 1, nodes, query);
            }
        }
    }

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

    // === Undo/Redo Helpers ===

    /// Save current state before making changes
    pub fn save_undo_state(&mut self) {
        self.undo_manager.save_state(&self.query, self.cursor_pos);
    }

    /// Undo last change
    pub fn undo(&mut self) -> bool {
        if let Some(state) = self.undo_manager.undo(&self.query, self.cursor_pos) {
            self.query = state.text;
            self.cursor_pos = state.cursor_pos.min(self.query.len());
            self.message = Some(t!("undo").to_string());
            true
        } else {
            self.message = Some(t!("nothing_to_undo").to_string());
            false
        }
    }

    /// Redo last undone change
    pub fn redo(&mut self) -> bool {
        if let Some(state) = self.undo_manager.redo(&self.query, self.cursor_pos) {
            self.query = state.text;
            self.cursor_pos = state.cursor_pos.min(self.query.len());
            self.message = Some(t!("redo").to_string());
            true
        } else {
            self.message = Some(t!("nothing_to_redo").to_string());
            false
        }
    }

    // === Character Search (f/F/t/T) ===

    /// Find character forward (f command)
    /// Returns the new cursor position if found
    pub fn find_char_forward(&mut self, ch: char, till: bool) -> bool {
        let chars: Vec<char> = self.query.chars().collect();
        let start = self.cursor_pos + 1;
        
        for i in start..chars.len() {
            // Stop at newline - f/F only work within current line
            if chars[i] == '\n' {
                return false;
            }
            if chars[i] == ch {
                self.cursor_pos = if till { i.saturating_sub(1) } else { i };
                self.last_char_search = Some((ch, true, till));
                return true;
            }
        }
        false
    }

    /// Find character backward (F command)
    /// Returns the new cursor position if found
    pub fn find_char_backward(&mut self, ch: char, till: bool) -> bool {
        let chars: Vec<char> = self.query.chars().collect();
        
        if self.cursor_pos == 0 {
            return false;
        }
        
        for i in (0..self.cursor_pos).rev() {
            // Stop at newline - f/F only work within current line
            if chars[i] == '\n' {
                return false;
            }
            if chars[i] == ch {
                self.cursor_pos = if till { (i + 1).min(self.cursor_pos) } else { i };
                self.last_char_search = Some((ch, false, till));
                return true;
            }
        }
        false
    }

    /// Repeat last character search (;)
    pub fn repeat_char_search(&mut self) -> bool {
        if let Some((ch, forward, till)) = self.last_char_search {
            if forward {
                self.find_char_forward(ch, till)
            } else {
                self.find_char_backward(ch, till)
            }
        } else {
            false
        }
    }

    /// Repeat last character search in opposite direction (,)
    pub fn repeat_char_search_opposite(&mut self) -> bool {
        if let Some((ch, forward, till)) = self.last_char_search {
            // Search in opposite direction
            if forward {
                self.find_char_backward(ch, till)
            } else {
                self.find_char_forward(ch, till)
            }
        } else {
            false
        }
    }
}
