//! Layout management

use crate::app::{App, ActivePanel, SPINNER_FRAMES};
use crate::ui::{DefaultTheme, draw_query_editor, draw_results_table, draw_schema_explorer, draw_history_panel, draw_completion_popup};
use ratatui::prelude::*;
use ratatui::widgets::{Block, Borders, Paragraph, Clear};
use rust_i18n::t;

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
            Constraint::Length(20),  // Logo/title
            Constraint::Min(20),     // Connection info
            Constraint::Length(25),  // Quick hints
        ])
        .split(area);

    // Logo/Title
    let logo = Paragraph::new(vec![
        Line::from(vec![
            Span::styled("╔═════════════════╗", DefaultTheme::title()),
        ]),
        Line::from(vec![
            Span::styled("║ ", DefaultTheme::title()),
            Span::styled("SQL Terminal UI ", Style::default().fg(DefaultTheme::TEXT)),
            Span::styled("║", DefaultTheme::title()),
        ]),
        Line::from(vec![
            Span::styled("╚═════════════════╝", DefaultTheme::title()),
        ]),
    ])
    .style(DefaultTheme::header());
    f.render_widget(logo, header_chunks[0]);

    // Connection info
    let conn_info = if let Some(ref db) = app.db {
        let database = db.database_name().replace("Evermart", "Checkout");
        let backend_label = db.backend().to_string();
        Paragraph::new(vec![
            Line::from(""),
            Line::from(vec![
                Span::styled("● ", DefaultTheme::success()),
                Span::styled(database, DefaultTheme::normal_text()),
                Span::styled(" · ", DefaultTheme::dim_text()),
                Span::styled(backend_label, DefaultTheme::dim_text()),
            ]),
            Line::from(""),
        ])
    } else {
        Paragraph::new(vec![
            Line::from(""),
            Line::from(vec![
                Span::styled("○ ", DefaultTheme::dim_text()),
                Span::styled(t!("disconnected").to_string(), DefaultTheme::dim_text()),
            ]),
            Line::from(""),
        ])
    }
    .style(DefaultTheme::header());
    f.render_widget(conn_info, header_chunks[1]);

    // Quick hints (instead of mode indicator)
    let hints = Paragraph::new(vec![
        Line::from(""),
        Line::from(vec![
            Span::styled("Ctrl+E", DefaultTheme::info()),
            Span::styled(format!(":{} ", t!("execute")), DefaultTheme::dim_text()),
            Span::styled("F1", DefaultTheme::info()),
            Span::styled(format!(":{} ", t!("help")), DefaultTheme::dim_text()),
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
            Constraint::Percentage(60),  // Query editor
            Constraint::Percentage(40),  // Results
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
    
    // Draw completion popup over the query editor (must be after query editor)
    if is_query_active && app.completion.visible {
        draw_completion_popup(f, app, left_chunks[0]);
    }
}

/// Draw the status bar
fn draw_status_bar(f: &mut Frame, app: &App, area: Rect) {
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Min(20),      // Messages
            Constraint::Length(78),   // Status info
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
        Paragraph::new(Span::styled(t!("query_placeholder").to_string(), DefaultTheme::dim_text()))
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
    let popup_area = centered_rect(60, 70, area);

    // Clear the area
    f.render_widget(Clear, popup_area);

    // Outer block
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(DefaultTheme::popup_border())
        .title(Span::styled(format!(" {} ", t!("help_title")), DefaultTheme::title()))
        .style(DefaultTheme::popup());
    let inner = block.inner(popup_area);
    f.render_widget(block, popup_area);

    // Header centered
    let header_area = Rect { height: 2, ..inner };
    let header = Paragraph::new(vec![
        Line::from(Span::styled(t!("help_header").to_string(), DefaultTheme::title())),
        Line::from(""),
    ]);
    f.render_widget(header, header_area);

    // Split remaining area into 2 columns
    let content_area = Rect {
        y: inner.y + 2,
        height: inner.height.saturating_sub(3),
        ..inner
    };
    let columns = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(content_area);

    // Left column: Global + Query Editor
    let left_text = vec![
        Line::from(Span::styled(t!("help_rule_global").to_string(), DefaultTheme::info())),
        Line::from(""),
        Line::from(t!("help_quote1").to_string()),
        Line::from(t!("help_quote2").to_string()),
        Line::from(t!("help_quote3").to_string()),
        Line::from(t!("help_quote4").to_string()),
        Line::from(t!("help_quote5").to_string()),
        Line::from(t!("help_quote6").to_string()),
        Line::from(t!("help_quote7").to_string()),
        Line::from(t!("help_quote8").to_string()),
        Line::from(t!("help_quote9").to_string()),
        Line::from(t!("help_quote10").to_string()),
        Line::from(t!("help_quote11").to_string()),
        Line::from(t!("help_quote12").to_string()),
        Line::from(""),
        Line::from(Span::styled(t!("help_rule_query_editor").to_string(), DefaultTheme::info())),
        Line::from(""),
        Line::from(t!("help_quote13").to_string()),
        Line::from(t!("help_quote14").to_string()),
        Line::from(t!("help_quote15").to_string()),
        Line::from(t!("help_quote16").to_string()),
        Line::from(t!("help_quote17").to_string()),
        Line::from(t!("help_quote18").to_string()),
        Line::from(t!("help_quote19").to_string()),
        Line::from(t!("help_quote20").to_string()),
        Line::from(t!("help_quote21").to_string()),
    ];
    f.render_widget(Paragraph::new(left_text), columns[0]);

    // Right column: Results + Schema + History
    let right_text = vec![
        Line::from(Span::styled(t!("help_rule_results").to_string(), DefaultTheme::info())),
        Line::from(""),
        Line::from(t!("help_quote22").to_string()),
        Line::from(t!("help_quote23").to_string()),
        Line::from(t!("help_quote24").to_string()),
        Line::from(t!("help_quote25").to_string()),
        Line::from(t!("help_quote26").to_string()),
        Line::from(t!("help_quote27").to_string()),
        Line::from(t!("help_quote28").to_string()),
        Line::from(t!("help_quote29").to_string()),
        Line::from(""),
        Line::from(Span::styled(t!("help_rule_schema").to_string(), DefaultTheme::info())),
        Line::from(""),
        Line::from(t!("help_quote30").to_string()),
        Line::from(t!("help_quote31").to_string()),
        Line::from(t!("help_quote32").to_string()),
        Line::from(""),
        Line::from(Span::styled(t!("help_rule_history").to_string(), DefaultTheme::info())),
        Line::from(""),
        Line::from(t!("help_quote33").to_string()),
    ];
    f.render_widget(Paragraph::new(right_text), columns[1]);

    // Footer
    let footer_area = Rect {
        y: content_area.y + content_area.height,
        height: 1,
        ..inner
    };
    let footer = Paragraph::new(Line::from(
        "\"Mais um dia para provar que o rock não morreu\"".to_string(),
    ));
    f.render_widget(footer, footer_area);
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
