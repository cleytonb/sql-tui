//! UI rendering module

mod theme;
mod layout;
mod widgets;

pub use theme::*;
pub use layout::*;
pub use widgets::*;

use crate::app::{App, SPINNER_FRAMES};
use ratatui::prelude::*;
use ratatui::widgets::{Block, Borders, Clear, Paragraph};

/// Main draw function
pub fn draw(f: &mut Frame, app: &mut App) {
    let size = f.size();

    // Draw main layout
    draw_layout(f, app, size);

    // Draw loading popup if active
    if app.is_loading {
        draw_loading_popup(f, app, size);
    }

    // Draw help popup if active
    if app.show_help {
        draw_help_popup(f, size);
    }
}

/// Draw loading spinner popup
fn draw_loading_popup(f: &mut Frame, app: &App, area: Rect) {
    let popup_width = 30;
    let popup_height = 5;

    let popup_area = Rect {
        x: (area.width.saturating_sub(popup_width)) / 2,
        y: (area.height.saturating_sub(popup_height)) / 2,
        width: popup_width.min(area.width),
        height: popup_height.min(area.height),
    };

    f.render_widget(Clear, popup_area);

    let spinner = SPINNER_FRAMES[app.spinner_frame];
    let loading_text = vec![
        Line::from(""),
        Line::from(vec![
            Span::styled(
                format!("  {}  Executando query...  ", spinner),
                Style::default()
                    .fg(DefaultTheme::GOLD)
                    .add_modifier(Modifier::BOLD),
            ),
        ]),
        Line::from(""),
    ];

    let loading = Paragraph::new(loading_text)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(DefaultTheme::PRIMARY))
                .style(Style::default().bg(DefaultTheme::BG_PANEL)),
        )
        .alignment(Alignment::Center);

    f.render_widget(loading, popup_area);
}
