//! Schema explorer keyboard handlers

use crate::app::{App, ActivePanel, SchemaNodeType};
use anyhow::Result;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

impl App {
    /// Schema explorer handler
    pub(crate) async fn handle_schema(&mut self, key: KeyEvent) -> Result<()> {
        // If search mode is active, process search input
        if self.show_search_schema {
            match key.code {
                KeyCode::Esc => {
                    self.show_search_schema = false;
                    self.schema_search_query.clear();
                    self.schema_selected = 0;
                    self.schema_scroll_offset = 0;
                }
                KeyCode::Enter => {
                    self.show_search_schema = false;
                    // Keep filter active
                }
                KeyCode::Backspace => {
                    self.schema_search_query.pop();
                    self.schema_selected = 0;
                    self.schema_scroll_offset = 0;
                }
                KeyCode::Char(c) => {
                    self.schema_search_query.push(c);
                    self.schema_selected = 0;
                    self.schema_scroll_offset = 0;
                }
                KeyCode::Up | KeyCode::Down => {
                    // Allow navigation even during search
                    let max = self.get_visible_schema_nodes().len().saturating_sub(1);
                    if key.code == KeyCode::Up {
                        self.schema_selected = self.schema_selected.saturating_sub(1);
                    } else if self.schema_selected < max {
                        self.schema_selected += 1;
                    }
                }
                _ => {}
            }
            return Ok(());
        }

        match key.code {
            KeyCode::Up | KeyCode::Char('k') => {
                self.schema_selected = self.schema_selected.saturating_sub(1);
            }
            KeyCode::Down | KeyCode::Char('j') => {
                let max = self.get_visible_schema_nodes().len().saturating_sub(1);
                if self.schema_selected < max {
                    self.schema_selected += 1;
                }
            }
            // Ctrl+D = Smooth scroll down
            KeyCode::Char('d') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                self.pending_scroll += 10;
                return Ok(());
            }
            // Ctrl+U = Smooth scroll up
            KeyCode::Char('u') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                self.pending_scroll -= 10;
                return Ok(());
            }
            // Fetch source
            KeyCode::Char('s') => {
                let visible = self.get_visible_schema_nodes();
                if let Some((_, node)) = visible.get(self.schema_selected) {
                    self.fetch_source(node.name.clone());
                }
            }
            KeyCode::Enter | KeyCode::Char(' ') => {
                let visible = self.get_visible_schema_nodes();
                if let Some((_, node)) = visible.get(self.schema_selected) {
                    if !node.children.is_empty() || node.node_type == SchemaNodeType::Folder {
                        self.toggle_schema_node();
                    } else {
                        self.insert_schema_object().await;
                    }
                }
            }
            KeyCode::Char('/') => {
                self.show_search_schema = true;
                self.schema_search_query.clear();
            }
            KeyCode::Esc => {
                // If search is active, clear it first
                if !self.schema_search_query.is_empty() {
                    self.schema_search_query.clear();
                    self.schema_selected = 0;
                    self.schema_scroll_offset = 0;
                }
            }
            _ => {}
        }
        Ok(())
    }

    /// Fetch source code for a schema object (placeholder)
    pub(crate) fn fetch_source(&mut self, _object_name: String) {
        // TODO: Implement fetching source code for stored procedures, views, etc.
    }
}
