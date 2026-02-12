//! Connection modal widget
//!
//! Displays a modal for managing database connections with a list on the left
//! and a form on the right.

use crate::app::{App, ConnectionModalFocus};
use crate::db::DatabaseBackend;
use crate::ui::DefaultTheme;
use ratatui::prelude::*;
use ratatui::widgets::{Block, Borders, Clear, List, ListItem, Paragraph};
use rust_i18n::t;

/// Draw the connection modal
pub fn draw_connection_modal(f: &mut Frame, app: &App, area: Rect) {
    let modal_area = if app.connection_modal_focus == ConnectionModalFocus::List {
        centered_rect(20, 40, area)
    } else {
        centered_rect(30, 50, area)
    };

    // Clear the background
    f.render_widget(Clear, modal_area);

    // Modal block
    let modal_block = Block::default()
        .title(" Conexões ")
        .title_style(DefaultTheme::title())
        .borders(Borders::ALL)
        .border_style(DefaultTheme::popup_border())
        .title_bottom(Line::from(if app.connection_modal_focus == ConnectionModalFocus::List {
            "[E] Editar [Enter] Conectar".to_string()
        } else {
            let backend_hint = match app.connection_form.backend {
                DatabaseBackend::SqlServer => "[Tab] Campos [Ctrl+T] SQLite",
                DatabaseBackend::Sqlite => "[Tab] Campos [Ctrl+T] SQL Server",
            };
            format!("[Esc] Voltar {} [Enter] Salvar", backend_hint)
        }).right_aligned())
        .style(DefaultTheme::popup());

    let inner = modal_block.inner(modal_area);
    f.render_widget(modal_block, modal_area);

    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(100)])
        .split(inner);

    if app.connection_modal_focus == ConnectionModalFocus::List {
        draw_connection_list(f, app, chunks[0]);
    } else {
        draw_connection_form(f, app, chunks[0]);
    }
}

/// Draw the connection list on the left
fn draw_connection_list(f: &mut Frame, app: &App, area: Rect) {
    let is_focused = app.connection_modal_focus == ConnectionModalFocus::List;

    let block = Block::default()
        .borders(Borders::RIGHT)
        .border_style(if is_focused {
            DefaultTheme::active_border()
        } else {
            DefaultTheme::inactive_border()
        });

    let inner = block.inner(area);
    f.render_widget(block, area);

    let mut items: Vec<ListItem> = Vec::new();

    for (i, conn) in app.app_config.connections.iter().enumerate() {
        let is_selected = i == app.connection_list_selected;
        let prefix = if is_selected { "▶ " } else { "  " };

        let style = if is_selected {
            Style::default().fg(DefaultTheme::GOLD).add_modifier(Modifier::BOLD)
        } else {
            DefaultTheme::normal_text()
        };

        let backend_tag = match conn.backend {
            DatabaseBackend::SqlServer => "",
            DatabaseBackend::Sqlite => " [SQLite]",
        };

        items.push(ListItem::new(Line::from(vec![
            Span::styled(prefix, style),
            Span::styled(&conn.name, style),
            Span::styled(backend_tag, DefaultTheme::dim_text()),
        ])));
    }

    // Separator
    items.push(ListItem::new(Line::from(Span::styled(
        "───────────────────────",
        DefaultTheme::dim_text(),
    ))));

    // "Create new" option
    let create_new_idx = app.app_config.connections.len();
    let is_create_selected = app.connection_list_selected >= create_new_idx;
    let create_style = if is_create_selected {
        Style::default().fg(DefaultTheme::SUCCESS).add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(DefaultTheme::SUCCESS)
    };
    let create_prefix = if is_create_selected { "▶ " } else { "  " };

    items.push(ListItem::new(Line::from(vec![
        Span::styled(create_prefix, create_style),
        Span::styled(t!("create_new_connection").to_string(), create_style),
    ])));

    let list = List::new(items);
    f.render_widget(list, inner);
}

/// Draw the connection form on the right
fn draw_connection_form(f: &mut Frame, app: &App, area: Rect) {
    let is_focused = app.connection_modal_focus == ConnectionModalFocus::Form;

    let block = Block::default()
        .borders(Borders::NONE)
        .style(DefaultTheme::popup());

    let inner = block.inner(area);
    f.render_widget(block, area);

    if app.app_config.connections.is_empty() && !app.is_create_new_selected() {
        let placeholder = Paragraph::new(vec![
            Line::from(""),
            Line::from(""),
            Line::from(Span::styled(
                t!("select_or_create").to_string(),
                DefaultTheme::dim_text(),
            )),
        ])
        .alignment(Alignment::Center);
        f.render_widget(placeholder, inner);
        return;
    }

    let form = &app.connection_form;
    let focus_idx = app.connection_form_focus;
    let num_fields = form.field_count();

    // Layout for form fields
    let field_height = 2u16;
    let mut constraints = Vec::new();

    // Backend selector row
    constraints.push(Constraint::Length(2));

    for _ in 0..num_fields {
        constraints.push(Constraint::Length(field_height));
    }
    constraints.push(Constraint::Length(2)); // Spacing
    constraints.push(Constraint::Length(2)); // Hint
    constraints.push(Constraint::Min(0));    // Remaining space

    let field_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints(constraints)
        .margin(1)
        .split(inner);

    // Draw backend selector
    let backend_label = match form.backend {
        DatabaseBackend::SqlServer => "Driver: SQL Server",
        DatabaseBackend::Sqlite => "Driver: SQLite",
    };
    let backend_style = Style::default().fg(DefaultTheme::GOLD).add_modifier(Modifier::BOLD);
    let backend_para = Paragraph::new(backend_label).style(backend_style);
    f.render_widget(backend_para, field_chunks[0]);

    // Draw each field
    for i in 0..num_fields {
        draw_form_field(
            f,
            form.get_field_label(i),
            form.get_field(i),
            i == focus_idx && is_focused,
            form.is_password_field(i),
            field_chunks[i + 1],  // +1 because index 0 is backend selector
        );
    }

    // Draw hint
    let hint_idx = num_fields + 2; // backend + fields + spacing
    if hint_idx < field_chunks.len() {
        let hint_style = if form.is_valid() {
            DefaultTheme::success()
        } else {
            DefaultTheme::dim_text()
        };

        if !form.is_valid() {
            let hint_text = t!("fill_required_fields").to_string();
            let hint = Paragraph::new(Line::from(vec![
                Span::styled("  ", Style::default()),
                Span::styled(hint_text, hint_style),
            ]));
            f.render_widget(hint, field_chunks[hint_idx]);
        }
    }
}

/// Draw a single form field
fn draw_form_field(f: &mut Frame, label: &str, value: &str, is_focused: bool, is_password: bool, area: Rect) {
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Length(12),
            Constraint::Min(20),
        ])
        .split(area);

    let label_style = if is_focused {
        Style::default().fg(DefaultTheme::GOLD).add_modifier(Modifier::BOLD)
    } else {
        DefaultTheme::normal_text()
    };
    let label_text = Paragraph::new(format!("{}:", label)).style(label_style);
    f.render_widget(label_text, chunks[0]);

    let display_value = if is_password && !value.is_empty() {
        "*".repeat(value.len())
    } else {
        value.to_string()
    };

    let input_style = if is_focused {
        Style::default()
            .fg(DefaultTheme::TEXT)
            .bg(DefaultTheme::BG_HIGHLIGHT)
    } else {
        Style::default().fg(DefaultTheme::TEXT)
    };

    let border_style = if is_focused {
        DefaultTheme::active_border()
    } else {
        DefaultTheme::inactive_border()
    };

    let display_with_cursor = if is_focused {
        format!("{}▏", display_value)
    } else {
        display_value
    };

    let input = Paragraph::new(display_with_cursor)
        .style(input_style)
        .block(
            Block::default()
                .borders(Borders::BOTTOM)
                .border_style(border_style),
        );
    f.render_widget(input, chunks[1]);
}

/// Helper to create a centered rect
fn centered_rect(percent_x: u16, percent_y: u16, r: Rect) -> Rect {
    let popup_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - percent_y) / 2),
            Constraint::Percentage(percent_y),
            Constraint::Percentage((100 - percent_y) / 2),
        ])
        .split(r);

    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Percentage(percent_x),
            Constraint::Percentage((100 - percent_x) / 2),
        ])
        .split(popup_layout[1])[1]
}
