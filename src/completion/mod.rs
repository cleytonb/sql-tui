//! SQL Autocomplete - Context-aware completion engine
//!
//! Provides intelligent code completion for SQL queries based on:
//! - Schema context (tables, views, procedures after schema.)
//! - SQL keywords (SELECT, FROM, WHERE, etc.)
//! - Statement context (EXEC suggests procedures, FROM suggests tables)

mod context;
mod candidates;

pub use context::{SqlContext, ObjectHint, TableRef, extract_context};
pub use candidates::{get_candidates, get_candidates_with_columns};

/// Completion state for the query editor
#[derive(Clone, Debug, Default)]
pub struct CompletionState {
    /// Whether the completion popup is visible
    pub visible: bool,
    /// List of completion candidates
    pub items: Vec<CompletionItem>,
    /// Currently selected item index
    pub selected: usize,
    /// Cursor position where completion was triggered
    pub trigger_pos: usize,
    /// The prefix being typed (for filtering)
    pub prefix: String,
}

impl CompletionState {
    /// Create a new empty completion state
    pub fn new() -> Self {
        Self::default()
    }

    /// Show completion with the given items
    pub fn show(&mut self, items: Vec<CompletionItem>, trigger_pos: usize, prefix: String) {
        self.items = items;
        self.visible = !self.items.is_empty();
        self.selected = 0;
        self.trigger_pos = trigger_pos;
        self.prefix = prefix;
    }

    /// Hide the completion popup
    pub fn hide(&mut self) {
        self.visible = false;
        self.items.clear();
        self.selected = 0;
    }

    /// Select the next item
    pub fn select_next(&mut self) {
        if !self.items.is_empty() {
            self.selected = (self.selected + 1) % self.items.len();
        }
    }

    /// Select the previous item
    pub fn select_prev(&mut self) {
        if !self.items.is_empty() {
            self.selected = if self.selected == 0 {
                self.items.len() - 1
            } else {
                self.selected - 1
            };
        }
    }

    /// Get the currently selected item
    pub fn get_selected(&self) -> Option<&CompletionItem> {
        self.items.get(self.selected)
    }

    /// Filter items by prefix
    pub fn filter(&mut self, prefix: &str) {
        self.prefix = prefix.to_string();
        let prefix_lower = prefix.to_lowercase();
        
        // Keep only items that match the prefix
        self.items.retain(|item| {
            item.label.to_lowercase().starts_with(&prefix_lower)
        });
        
        // Hide if no matches
        self.visible = !self.items.is_empty();
        
        // Reset selection if out of bounds
        if self.selected >= self.items.len() {
            self.selected = 0;
        }
    }
}

/// A single completion item
#[derive(Clone, Debug)]
pub struct CompletionItem {
    /// Display label (e.g., "Customers")
    pub label: String,
    /// Type of completion
    pub kind: CompletionKind,
    /// Text to insert when selected
    pub insert_text: String,
    /// Additional detail (e.g., schema name)
    pub detail: Option<String>,
}

impl CompletionItem {
    /// Create a new completion item
    pub fn new(label: impl Into<String>, kind: CompletionKind) -> Self {
        let label = label.into();
        Self {
            insert_text: label.clone(),
            label,
            kind,
            detail: None,
        }
    }

    /// Create a completion item with schema detail
    pub fn with_schema(label: impl Into<String>, kind: CompletionKind, schema: impl Into<String>) -> Self {
        let label = label.into();
        Self {
            insert_text: label.clone(),
            label,
            kind,
            detail: Some(schema.into()),
        }
    }

    /// Get the icon for this completion kind
    pub fn icon(&self) -> &'static str {
        self.kind.icon()
    }
}

/// Type of completion item
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CompletionKind {
    /// Combined column list for INSERT (highest priority)
    ColumnList,
    /// SQL keyword (SELECT, FROM, WHERE, etc.)
    Keyword,
    /// Database schema
    Schema,
    /// Table
    Table,
    /// View
    View,
    /// Stored procedure
    Procedure,
    /// Table/view column
    Column,
    /// SQL function
    Function,
    /// SQL variable (@Var)
    Variable,
}

impl CompletionKind {
    /// Get the display icon for this kind
    pub fn icon(&self) -> &'static str {
        match self {
            CompletionKind::ColumnList => "󰠷 ",
            CompletionKind::Keyword => "󰌆 ",
            CompletionKind::Schema => " ",
            CompletionKind::Table => "󰓫 ",
            CompletionKind::View => "󰈈 ",
            CompletionKind::Procedure => " ",
            CompletionKind::Column => " ",
            CompletionKind::Function => "󰊕 ",
            CompletionKind::Variable => "󰫧 ",
        }
    }

    /// Get a short label for this kind
    pub fn label(&self) -> &'static str {
        match self {
            CompletionKind::ColumnList => "cl",
            CompletionKind::Keyword => "kw",
            CompletionKind::Schema => "sc",
            CompletionKind::Table => "tb",
            CompletionKind::View => "vw",
            CompletionKind::Procedure => "sp",
            CompletionKind::Column => "cl",
            CompletionKind::Function => "fn",
            CompletionKind::Variable => "vr",
        }
    }
}
