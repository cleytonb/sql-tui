//! History panel widget

use crate::app::App;
use crate::ui::DefaultTheme;
use ratatui::prelude::*;
use ratatui::widgets::{Block, Borders, List, ListItem, ListState};

/// Draw the history panel
pub fn draw_history_panel(f: &mut Frame, app: &mut App, area: Rect, active: bool) {
    let border_style = if active {
        DefaultTheme::active_border()
    } else {
        DefaultTheme::inactive_border()
    };

    let title = if active { " Histórico [<Cmd>h] ▪ " } else { " Histórico [<Cmd>h] " };

    let entries = app.history.entries();
    let items: Vec<ListItem> = entries
        .iter()
        .rev()
        .enumerate()
        .map(|(idx, entry)| {
            let time = entry.timestamp.format("%H:%M:%S").to_string();
            let query_preview: String = entry
                .query
                .chars()
                .take(50)
                .filter(|c| !c.is_control())
                .collect();
            let query_preview = if entry.query.len() > 50 {
                format!("{}...", query_preview)
            } else {
                query_preview
            };

            let row_info = entry.row_count.map(|r| format!(" ({} rows)", r)).unwrap_or_default();

            let style = if active && idx == app.history_selected {
                DefaultTheme::selected()
            } else {
                DefaultTheme::normal_text()
            };

            ListItem::new(format!("{} │ {}{}", time, query_preview, row_info)).style(style)
        })
        .collect();

    let list = List::new(items)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(border_style)
                .title(Span::styled(
                    format!("{} ({}) ", title, app.history.len()),
                    DefaultTheme::title(),
                )),
        );

    // Calcula a altura visível (área - bordas)
    let visible_height = area.height.saturating_sub(2) as usize;

    // Ajusta o offset de scroll para manter o item selecionado visível
    if app.history_selected < app.history_scroll_offset {
        app.history_scroll_offset = app.history_selected;
    } else if app.history_selected >= app.history_scroll_offset + visible_height {
        app.history_scroll_offset = app.history_selected
            .saturating_sub(visible_height.saturating_sub(1));
    }

    let mut list_state = ListState::default()
        .with_selected(Some(app.history_selected))
        .with_offset(app.history_scroll_offset);
    f.render_stateful_widget(list, area, &mut list_state);
}
