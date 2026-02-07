//! Connection modal event handler

use crate::app::{App, ConnectionModalFocus};
use crate::config::ConnectionForm;
use anyhow::Result;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

impl App {
    /// Handle keyboard input for the connection modal
    pub async fn handle_connection_modal(&mut self, key: KeyEvent) -> Result<()> {
        match key.code {
            // Escape - close modal (only if already connected)
            KeyCode::Esc => {
                if self.is_connected() {
                    self.show_connection_modal = false;
                }
            }

            // Enter - either select connection or save and connect
            KeyCode::Enter => {
                self.handle_connection_enter().await?;
            }

            // Navigation and input
            KeyCode::Up => self.handle_connection_up(),
            KeyCode::Down => self.handle_connection_down(),
            KeyCode::Char('k') if self.connection_modal_focus == ConnectionModalFocus::List => self.handle_connection_up(),
            KeyCode::Char('j') if self.connection_modal_focus == ConnectionModalFocus::List => self.handle_connection_down(),
            KeyCode::Tab if self.connection_modal_focus == ConnectionModalFocus::Form && key.modifiers.contains(KeyModifiers::SHIFT) => self.handle_connection_up(),
            KeyCode::Tab if self.connection_modal_focus == ConnectionModalFocus::Form => self.handle_connection_down(),
            KeyCode::Char(c) => self.handle_connection_char(c),
            KeyCode::Backspace => self.handle_connection_backspace(),

            _ => {}
        }

        return Ok(())
    }

    /// Handle Enter key in connection modal
    async fn handle_connection_enter(&mut self) -> Result<()> {
        match self.connection_modal_focus {
            ConnectionModalFocus::List => {
                // Select the connection or prepare new form
                if self.is_create_new_selected() {
                    // Switch to form for new connection
                    self.connection_form = ConnectionForm::new_empty();
                    self.connection_form_focus = 0;
                    self.connection_modal_focus = ConnectionModalFocus::Form;
                } else if let Some(conn) = self.get_selected_connection().cloned() {
                    // Load existing connection into form
                    match self.connect(&conn).await {
                        Ok(_) => {
                            // Connection successful, modal will close
                            self.message = Some(format!("Conectado a {}", conn.name));
                        }
                        Err(e) => {
                            self.error = Some(format!("Erro ao conectar: {}", e));
                        }
                    }
                }
            }
            ConnectionModalFocus::Form => {
                // Try to save and connect
                if let Some(config) = self.connection_form.to_config() {
                    // Save to config
                    self.app_config.add_connection(config.clone());
                    let _ = self.app_config.save();

                    // Try to connect
                    match self.connect(&config).await {
                        Ok(_) => {
                            // Connection successful, modal will close
                            self.message = Some(format!("Conectado a {}", config.name));
                            self.connection_modal_focus = ConnectionModalFocus::List;
                        }
                        Err(e) => {
                            self.error = Some(format!("Erro ao conectar: {}", e));
                        }
                    }
                }
            }
        }
        Ok(())
    }

    /// Handle Up arrow in connection modal
    fn handle_connection_up(&mut self) {
        match self.connection_modal_focus {
            ConnectionModalFocus::List => {
                if self.connection_list_selected > 0 {
                    self.connection_list_selected -= 1;
                    // Update form when selection changes
                    self.update_form_from_selection();
                }
            }
            ConnectionModalFocus::Form => {
                if self.connection_form_focus > 0 {
                    self.connection_form_focus -= 1;
                }
            }
        }
    }

    /// Handle Down arrow in connection modal
    fn handle_connection_down(&mut self) {
        match self.connection_modal_focus {
            ConnectionModalFocus::List => {
                let max = self.connection_list_len().saturating_sub(1);
                if self.connection_list_selected < max {
                    self.connection_list_selected += 1;
                    // Update form when selection changes
                    self.update_form_from_selection();
                }
            }
            ConnectionModalFocus::Form => {
                if self.connection_form_focus < ConnectionForm::FIELD_COUNT - 1 {
                    self.connection_form_focus += 1;
                }
            }
        }
    }

    /// Handle character input in connection modal
    fn handle_connection_char(&mut self, c: char) {
        if self.connection_modal_focus == ConnectionModalFocus::Form {
            if let Some(field) = self.connection_form.get_field_mut(self.connection_form_focus) {
                field.push(c);
            }
        } else if c == 'e' {
            if let Some(conn) = self.get_selected_connection().cloned() {
                // Load existing connection into form
                self.connection_form = ConnectionForm::from_config(&conn);
                self.connection_form_focus = 0;
                self.connection_modal_focus = ConnectionModalFocus::Form;
            }
        }
    }

    /// Handle backspace in connection modal
    fn handle_connection_backspace(&mut self) {
        if self.connection_modal_focus == ConnectionModalFocus::Form {
            if let Some(field) = self.connection_form.get_field_mut(self.connection_form_focus) {
                field.pop();
            }
        }
    }

    /// Update form based on current list selection
    pub fn update_form_from_selection(&mut self) {
        if self.is_create_new_selected() {
            self.connection_form = ConnectionForm::new_empty();
        } else if let Some(conn) = self.get_selected_connection().cloned() {
            self.connection_form = ConnectionForm::from_config(&conn);
        }
    }
}
