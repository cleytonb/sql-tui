//! Layout management

use crate::app::{App, ActivePanel, SPINNER_FRAMES, InputMode};
use crate::ui::{DefaultTheme, draw_query_editor, draw_results_table, draw_schema_explorer, draw_history_panel};
use ratatui::prelude::*;
use ratatui::widgets::{Block, Borders, Paragraph, Clear};

/// Draw the main layout
pub fn draw_layout(f: &mut Frame, app: &mut App, area: Rect) {
    // Main vertical layout: header, content, status bar
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),   // Header
            Constraint::Min(10),     // Content
            Constraint::Length(1),   // Status bar
        ])
        .split(area);

    // Draw header
    draw_header(f, app, chunks[0]);

    // Draw main content (horizontal split)
    draw_content(f, app, chunks[1]);

    // Draw status bar
    draw_status_bar(f, app, chunks[2]);
}

/// Draw the header with Alrajhi Bank branding
fn draw_header(f: &mut Frame, app: &App, area: Rect) {
    let header_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Length(40),  // Logo/title
            Constraint::Min(20),     // Connection info
            Constraint::Length(25),  // Quick hints
        ])
        .split(area);

    // Logo/Title
    let logo = Paragraph::new(vec![
        Line::from(vec![
            Span::styled("╔══════════════════════════╗", DefaultTheme::title()),
        ]),
        Line::from(vec![
            Span::styled("║ ", DefaultTheme::title()),
            Span::styled("🏦 CRYPTONICS ", Style::default().fg(DefaultTheme::GOLD).add_modifier(Modifier::BOLD)),
            Span::styled("SQL Studio ", Style::default().fg(DefaultTheme::TEXT)),
            Span::styled("║", DefaultTheme::title()),
        ]),
        Line::from(vec![
            Span::styled("╚══════════════════════════╝", DefaultTheme::title()),
        ]),
    ])
    .style(DefaultTheme::header());
    f.render_widget(logo, header_chunks[0]);

    // Connection info (app.db.config.database tem que ter replace de Evermart para Checkout)
    let database = app.db.config.database.replace("Evermart", "Checkout");
    let conn_info = Paragraph::new(vec![
        Line::from(""),
        Line::from(vec![
            Span::styled("● ", DefaultTheme::success()),
            Span::styled(&database, DefaultTheme::normal_text()),
            Span::styled(" @ ", DefaultTheme::dim_text()),
            Span::styled(&app.db.config.host, DefaultTheme::dim_text()),
        ]),
        Line::from(""),
    ])
    .style(DefaultTheme::header());
    f.render_widget(conn_info, header_chunks[1]);

    // Quick hints (instead of mode indicator)
    let hints = Paragraph::new(vec![
        Line::from(""),
        Line::from(vec![
            Span::styled("Ctrl+E", DefaultTheme::info()),
            Span::styled(":Executar ", DefaultTheme::dim_text()),
            Span::styled("F1", DefaultTheme::info()),
            Span::styled(":Ajuda ", DefaultTheme::dim_text()),
        ]),
        Line::from(""),
    ])
    .style(DefaultTheme::header())
    .alignment(Alignment::Right);
    f.render_widget(hints, header_chunks[2]);
}

/// Draw main content area
fn draw_content(f: &mut Frame, app: &mut App, area: Rect) {
    // Horizontal split: left (query + results), right (schema + history)
    let h_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(70),  // Main area
            Constraint::Percentage(30),  // Side panels
        ])
        .split(area);

    // Left side: Query editor + Results
    let left_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage(35),  // Query editor
            Constraint::Percentage(65),  // Results
        ])
        .split(h_chunks[0]);

    // Right side: Schema explorer + History
    let right_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage(60),  // Schema explorer
            Constraint::Percentage(40),  // History
        ])
        .split(h_chunks[1]);

    // Draw each panel - query editor needs mutable access for scroll updates
    let is_query_active = app.active_panel == ActivePanel::QueryEditor;
    let is_results_active = app.active_panel == ActivePanel::Results;
    let is_schema_active = app.active_panel == ActivePanel::SchemaExplorer;
    let is_history_active = app.active_panel == ActivePanel::History;

    draw_query_editor(f, app, left_chunks[0], is_query_active);
    draw_results_table(f, app, left_chunks[1], is_results_active);
    draw_schema_explorer(f, app, right_chunks[0], is_schema_active);
    draw_history_panel(f, app, right_chunks[1], is_history_active);
}

/// Draw the status bar
fn draw_status_bar(f: &mut Frame, app: &App, area: Rect) {
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Min(20),      // Messages
            Constraint::Length(76),   // Status info
        ])
        .split(area);

    // Messages (error or success)
    let message = if let Some(ref err) = app.error {
        Paragraph::new(Span::styled(
            format!("❌ {}", err),
            DefaultTheme::error(),
        ))
    } else if let Some(ref msg) = app.message {
        Paragraph::new(Span::styled(
            format!("✓ {}", msg),
            DefaultTheme::success(),
        ))
    } else if app.is_loading {
        let spinner = SPINNER_FRAMES[app.spinner_frame];
        Paragraph::new(Span::styled(
            format!("{} Executando query...", spinner),
            DefaultTheme::warning(),
        ))
    } else {
        Paragraph::new(Span::styled("Monte a query, CTRL+E para executar", DefaultTheme::dim_text()))
    };

    f.render_widget(message.style(DefaultTheme::status_bar()), chunks[0]);

    // Status info
    let status_info = format!(
        " {} ",
        app.status
    );
    let status = Paragraph::new(status_info)
        .style(DefaultTheme::status_bar())
        .alignment(Alignment::Center);
    f.render_widget(status, chunks[1]);
}

/// Draw help popup
pub fn draw_help_popup(f: &mut Frame, area: Rect) {
    let popup_area = centered_rect(30, 30, area);

    // Clear the area
    f.render_widget(Clear, popup_area);

    let help_text = vec![
        Line::from(Span::styled("🏦 CRYPTONICS SQL STUDIO - AJUDA", DefaultTheme::title())),
        Line::from(""),
        Line::from(Span::styled("═══ REGRA ÚNICA ═══", DefaultTheme::info())),
        Line::from(""),
        Line::from("Vencedores não precisam de ajuda"),
        Line::from(""),
        Line::from(""),
        Line::from("\"Mais um dia para provar que o rock não morreu - Mel\"")
    ];

    let help = Paragraph::new(help_text)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(DefaultTheme::popup_border())
                .title(Span::styled(" Ajuda ", DefaultTheme::title()))
                .style(DefaultTheme::popup()),
        )
        .wrap(ratatui::widgets::Wrap { trim: false });

    f.render_widget(help, popup_area);
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
