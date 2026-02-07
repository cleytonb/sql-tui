//! Event handlers for the application

mod query_editor;
mod results;
mod schema;
mod history_handler;

use crate::app::{App, ActivePanel, ResultsTab, SPINNER_FRAMES, InputMode};
use anyhow::Result;
use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyModifiers, MouseEvent, MouseEventKind};
use ratatui::prelude::*;
use std::time::Duration;

impl App {
    /// Main event loop
    pub async fn run<B: Backend>(&mut self, terminal: &mut Terminal<B>) -> Result<()> {
        loop {
            // Check for query completion
            self.check_query_completion();

            // Process smooth scroll animation
            self.process_smooth_scroll();

            // Advance spinner animation when loading
            if self.is_loading {
                self.spinner_frame = (self.spinner_frame + 1) % SPINNER_FRAMES.len();
            }

            terminal.draw(|f| crate::ui::draw(f, self))?;

            // Use shorter poll time for animations (smooth scroll or loading spinner)
            let poll_duration = if self.is_loading || self.pending_scroll != 0 {
                Duration::from_millis(25)
            } else {
                Duration::from_millis(100)
            };

            if event::poll(poll_duration)? {
                match event::read()? {
                    Event::Key(key) => {
                        self.handle_key(key)?;
                    }
                    Event::Mouse(mouse) => {
                        self.handle_mouse(mouse)?;
                    }
                    _ => {}
                }
            }

            if self.should_quit {
                break;
            }
        }

        Ok(())
    }

    /// Process one step of smooth scroll animation
    fn process_smooth_scroll(&mut self) {
        if self.pending_scroll == 0 {
            return;
        }

        if self.pending_scroll > 0 {
            self.scroll_down(1);
            self.move_cursor_down();
            self.pending_scroll -= 1;
        } else {
            self.scroll_up(1);
            self.move_cursor_up();
            self.pending_scroll += 1;
        }
    }

    /// Handle keyboard input
    fn handle_key(&mut self, key: KeyEvent) -> Result<()> {
        // Don't process keys while loading (except quit)
        if self.is_loading {
            match (key.code, key.modifiers) {
                (KeyCode::Char('c'), KeyModifiers::CONTROL) |
                (KeyCode::Char('q'), KeyModifiers::CONTROL) => {
                    self.should_quit = true;
                }
                _ => {}
            }
            return Ok(());
        }

        // Clear messages on any keypress
        if key.code != KeyCode::Enter {
            self.message = None;
        }

        // Quit shortcuts - always work
        match (key.code, key.modifiers) {
            (KeyCode::Char('c'), KeyModifiers::CONTROL) |
            (KeyCode::Char('q'), KeyModifiers::CONTROL) => {
                self.should_quit = true;
                return Ok(());
            }
            _ => {}
        }

        // Help toggle
        if key.code == KeyCode::F(1) {
            self.show_help = !self.show_help;
            return Ok(());
        }

        if self.show_help {
            if key.code == KeyCode::Esc {
                self.show_help = false;
            }
            return Ok(());
        }

        // Esc no QueryEditor em modo Insert -> volta para Normal
        if key.code == KeyCode::Esc && self.active_panel == ActivePanel::QueryEditor && self.input_mode == InputMode::Insert {
            self.input_mode = InputMode::Normal;
            return Ok(());
        }

        // Tab in non-query panels switches panels
        if key.code == KeyCode::Tab && (self.active_panel != ActivePanel::QueryEditor || self.input_mode == InputMode::Normal) {
            self.active_panel = match self.active_panel {
                ActivePanel::QueryEditor => ActivePanel::Results,
                ActivePanel::Results => ActivePanel::SchemaExplorer,
                ActivePanel::SchemaExplorer => ActivePanel::History,
                ActivePanel::History => ActivePanel::QueryEditor,
            };
            return Ok(());
        }

        // 'q' in Normal mode -> switch to QueryEditor
        if key.code == KeyCode::Char('q') && self.input_mode == InputMode::Normal && !self.show_search_schema {
            self.active_panel = ActivePanel::QueryEditor;
            return Ok(());
        }

        // 'r' in Normal mode -> switch to Results
        if key.code == KeyCode::Char('r') && self.input_mode == InputMode::Normal && !self.show_search_schema {
            self.active_panel = ActivePanel::Results;
            return Ok(());
        }

        // 's' in Normal mode -> switch to SchemaExplorer
        if key.code == KeyCode::Char('s') && self.input_mode == InputMode::Normal && !self.show_search_schema {
            self.active_panel = ActivePanel::SchemaExplorer;
            return Ok(());
        }

        // Handle based on active panel
        match self.active_panel {
            ActivePanel::QueryEditor => self.handle_query_editor(key)?,
            ActivePanel::Results => self.handle_results(key)?,
            ActivePanel::SchemaExplorer => self.handle_schema(key)?,
            ActivePanel::History => self.handle_history(key)?,
        }

        Ok(())
    }

    /// Handle mouse input (scroll events)
    fn handle_mouse(&mut self, mouse: MouseEvent) -> Result<()> {
        // Don't process mouse while loading
        if self.is_loading {
            return Ok(());
        }

        match mouse.kind {
            MouseEventKind::ScrollUp => {
                self.scroll_up(3); // Scroll 3 lines at a time
            }
            MouseEventKind::ScrollDown => {
                self.scroll_down(3); // Scroll 3 lines at a time
            }
            _ => {}
        }

        Ok(())
    }

    /// Scroll up in the current panel
    pub(crate) fn scroll_up(&mut self, amount: usize) {
        match self.active_panel {
            ActivePanel::Results => {
                match self.results_tab {
                    ResultsTab::Data => {
                        self.results_selected = self.results_selected.saturating_sub(amount);
                    }
                    ResultsTab::Columns => {
                        self.results_selected = self.results_selected.saturating_sub(amount);
                    }
                    ResultsTab::Stats => {
                        // Stats view doesn't need scrolling (it's short)
                    }
                }
            }
            ActivePanel::SchemaExplorer => {
                self.schema_selected = self.schema_selected.saturating_sub(amount);
            }
            ActivePanel::History => {
                self.history_selected = self.history_selected.saturating_sub(amount);
            }
            ActivePanel::QueryEditor => {
                self.query_scroll_y = self.query_scroll_y.saturating_sub(amount);
            }
        }
    }

    /// Scroll down in the current panel
    pub(crate) fn scroll_down(&mut self, amount: usize) {
        match self.active_panel {
            ActivePanel::Results => {
                match self.results_tab {
                    ResultsTab::Data => {
                        let max_rows = self.result.rows.len().saturating_sub(1);
                        self.results_selected = (self.results_selected + amount).min(max_rows);
                    }
                    ResultsTab::Columns => {
                        let max_cols = self.result.columns.len().saturating_sub(1);
                        self.results_selected = (self.results_selected + amount).min(max_cols);
                    }
                    ResultsTab::Stats => {
                        // Stats view doesn't need scrolling
                    }
                }
            }
            ActivePanel::SchemaExplorer => {
                let max = self.get_visible_schema_nodes().len().saturating_sub(1);
                self.schema_selected = (self.schema_selected + amount).min(max);
            }
            ActivePanel::History => {
                let max = self.history.len().saturating_sub(1);
                self.history_selected = (self.history_selected + amount).min(max);
            }
            ActivePanel::QueryEditor => {
                let max_scroll = self.query.lines().count().saturating_sub(1);
                self.query_scroll_y = (self.query_scroll_y + amount).min(max_scroll);
            }
        }
    }

    /// Move cursor up one line in query
    pub(crate) fn move_cursor_up(&mut self) {
        let text_before: String = self.query.chars().take(self.cursor_pos).collect();
        if let Some(last_newline) = text_before.rfind('\n') {
            let col = self.cursor_pos - last_newline - 1;
            let before_that: String = text_before.chars().take(last_newline).collect();
            if let Some(prev_newline) = before_that.rfind('\n') {
                let prev_line_len = last_newline - prev_newline - 1;
                self.cursor_pos = prev_newline + 1 + col.min(prev_line_len);
            } else {
                self.cursor_pos = col.min(last_newline);
            }
        }
    }

    /// Move cursor down one line in query
    pub(crate) fn move_cursor_down(&mut self) {
        let text_before: String = self.query.chars().take(self.cursor_pos).collect();
        let text_after: String = self.query.chars().skip(self.cursor_pos).collect();

        let col = if let Some(last_newline) = text_before.rfind('\n') {
            self.cursor_pos - last_newline - 1
        } else {
            self.cursor_pos
        };

        if let Some(next_newline) = text_after.find('\n') {
            let next_line_start = self.cursor_pos + next_newline + 1;
            let remaining: String = self.query.chars().skip(next_line_start).collect();
            let next_line_len = remaining.find('\n').unwrap_or(remaining.len());
            self.cursor_pos = next_line_start + col.min(next_line_len);
        }
    }
}
