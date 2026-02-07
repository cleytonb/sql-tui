//! Application state - core data structures and state management
//!
//! This module contains the main App struct and related types.
//! Business logic and async operations are in the actions module.

use crate::db::{DbConfig, DbConnection, QueryResult};
use crate::app::{QueryHistory, UndoManager};
use anyhow::Result;
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

/// Spinner animation frames
pub const SPINNER_FRAMES: &[&str] = &["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"];

/// Main application state
pub struct App {
    // === Database ===
    /// Database connection
    pub db: DbConnection,

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
    /// Show search input in schema explorer
    pub show_search_schema: bool,
    /// Search query for schema explorer
    pub schema_search_query: String,

    // === History ===
    /// Query history
    pub history: QueryHistory,
    /// History scroll position
    pub history_selected: usize,

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
            query_scroll_x: 0,
            query_scroll_y: 0,
            input_mode: InputMode::Insert,
            visual_anchor: 0,
            last_char_search: None,
            pending_char_search: None,
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
            show_search_schema: false,
            schema_search_query: String::new(),
            history: QueryHistory::new(1000),
            history_selected: 0,
            undo_manager: UndoManager::new(1000),
            command_mode: false,
            active_panel: ActivePanel::QueryEditor,
            should_quit: false,
            show_help: false,
            error: None,
            message: Some("Conectado ao SQL Server".to_string()),
            status: format!("Conectado | {}", short_version),
            server_version: short_version,
            command_buffer: String::new(),
            pending_scroll: 0,
        };

        // Load initial schema
        app.load_schema().await?;

        // Auto-execute default query to show results on startup
        app.execute_default_query().await;

        Ok(app)
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
            self.message = Some("Undo".to_string());
            true
        } else {
            self.message = Some("Nada para desfazer".to_string());
            false
        }
    }

    /// Redo last undone change
    pub fn redo(&mut self) -> bool {
        if let Some(state) = self.undo_manager.redo(&self.query, self.cursor_pos) {
            self.query = state.text;
            self.cursor_pos = state.cursor_pos.min(self.query.len());
            self.message = Some("Redo".to_string());
            true
        } else {
            self.message = Some("Nada para refazer".to_string());
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
