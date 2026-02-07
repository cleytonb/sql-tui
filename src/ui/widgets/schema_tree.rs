//! Schema explorer tree widget

use crate::app::{App, SchemaNodeType};
use crate::ui::DefaultTheme;
use ratatui::prelude::*;
use ratatui::widgets::{Block, Borders, List, ListItem, Paragraph};

/// Draw the schema explorer panel
pub fn draw_schema_explorer(f: &mut Frame, app: &App, area: Rect, active: bool) {
    let border_style = if active {
        DefaultTheme::active_border()
    } else {
        DefaultTheme::inactive_border()
    };

    // Título com indicador de busca ativa
    let title = if !app.schema_search_query.is_empty() {
        format!(" Schema [<Cmd>s] 🔍 {} ", app.schema_search_query)
    } else if active {
        " Schema [<Cmd>s] ▪ ".to_string()
    } else {
        " Schema [<Cmd>s] ".to_string()
    };

    // Se o modo de busca está ativo, reserva espaço para o input
    let (search_area, list_area) = if app.show_search_schema {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Length(3), Constraint::Min(1)])
            .split(area);
        (Some(chunks[0]), chunks[1])
    } else {
        (None, area)
    };

    // Renderiza o input de busca se ativo
    if let Some(search_area) = search_area {
        let search_input = Paragraph::new(Line::from(vec![
            Span::styled(" / ", Style::default().fg(DefaultTheme::PRIMARY)),
            Span::styled(&app.schema_search_query, DefaultTheme::normal_text()),
            Span::styled("█", Style::default().fg(DefaultTheme::PRIMARY)), // cursor
        ]))
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(DefaultTheme::PRIMARY))
                .title(Span::styled(" Search (Enter to confirm, Esc to cancel) ", DefaultTheme::info())),
        );
        f.render_widget(search_input, search_area);
    }

    let visible_nodes = app.get_visible_schema_nodes();

    let items: Vec<ListItem> = visible_nodes
        .iter()
        .enumerate()
        .map(|(idx, (depth, node))| {
            let indent = "  ".repeat(*depth);
            let icon = node.icon();
            let expand_indicator = if !node.children.is_empty() {
                if node.expanded { "▼ " } else { "▶ " }
            } else {
                "  "
            };

            let style = if active && idx == app.schema_selected {
                DefaultTheme::selected()
            } else {
                match node.node_type {
                    SchemaNodeType::Folder => DefaultTheme::info(),
                    SchemaNodeType::Table => DefaultTheme::normal_text(),
                    SchemaNodeType::View => DefaultTheme::dim_text(),
                    SchemaNodeType::Procedure => DefaultTheme::warning(),
                    SchemaNodeType::Function => DefaultTheme::warning(),
                    _ => DefaultTheme::normal_text(),
                }
            };

            // Destaca o texto que corresponde à busca
            let name = if !app.schema_search_query.is_empty() {
                highlight_search_match(&node.name, &app.schema_search_query)
            } else {
                vec![Span::styled(node.name.clone(), style)]
            };

            let mut spans = vec![
                Span::styled(format!("{}{}{} ", indent, expand_indicator, icon), style),
            ];
            
            if active && idx == app.schema_selected {
                // Se selecionado, usa o estilo de seleção para todo o texto
                spans.push(Span::styled(node.name.clone(), DefaultTheme::selected()));
            } else {
                spans.extend(name);
            }

            ListItem::new(Line::from(spans))
        })
        .collect();

    // Mostra contagem de resultados se há busca ativa
    let block_title = if !app.schema_search_query.is_empty() && !app.show_search_schema {
        format!("{} ({} results)", title, visible_nodes.len())
    } else {
        title
    };

    let list = List::new(items)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(border_style)
                .title(Span::styled(block_title, DefaultTheme::title())),
        )
        .highlight_style(DefaultTheme::selected());

    f.render_widget(list, list_area);
}

/// Highlight matching text in search results
fn highlight_search_match<'a>(text: &str, query: &str) -> Vec<Span<'a>> {
    let text_lower = text.to_lowercase();
    let query_lower = query.to_lowercase();
    
    if let Some(start) = text_lower.find(&query_lower) {
        let end = start + query.len();
        vec![
            Span::styled(text[..start].to_string(), DefaultTheme::normal_text()),
            Span::styled(
                text[start..end].to_string(),
                Style::default()
                    .fg(DefaultTheme::TEXT)
                    .bg(DefaultTheme::PRIMARY)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(text[end..].to_string(), DefaultTheme::normal_text()),
        ]
    } else {
        vec![Span::styled(text.to_string(), DefaultTheme::normal_text())]
    }
}
