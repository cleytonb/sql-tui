//! UI widgets for the application

use crate::app::{App, SchemaNodeType, ResultsTab, InputMode};
use crate::db::CellValue;
use crate::ui::DefaultTheme;
use ratatui::prelude::*;
use ratatui::widgets::{Block, Borders, List, ListItem, Paragraph, Row, Table, Scrollbar, ScrollbarOrientation, ScrollbarState, Cell};
use ratatui::layout::Margin;

/// Line number gutter width (4 chars + 1 separator)
const LINE_NUMBER_WIDTH: u16 = 5;

/// Draw the query editor panel with line numbers and scrolling
pub fn draw_query_editor(f: &mut Frame, app: &mut App, area: Rect, active: bool) {
    let border_style = if active {
        DefaultTheme::active_border()
    } else {
        DefaultTheme::inactive_border()
    };

    // Title with active and input mode indicator
    let mode_indicator = match app.input_mode {
        InputMode::Insert => "[INSERT]",
        InputMode::Visual => "[VISUAL]",
        InputMode::Normal => "",
        InputMode::Command => "[COMMAND]",
    };
    let title = format!(" Query [q] {} {} ", if active { "▪" } else { "" }, mode_indicator);

    // Create outer block
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(border_style)
        .title(Span::styled(title, DefaultTheme::title()));

    let inner_area = block.inner(area);
    f.render_widget(block, area);

    // Split inner area: line numbers | code
    if inner_area.width > LINE_NUMBER_WIDTH + 2 {
        let line_num_area = Rect {
            x: inner_area.x,
            y: inner_area.y,
            width: LINE_NUMBER_WIDTH,
            height: inner_area.height,
        };

        let code_area = Rect {
            x: inner_area.x + LINE_NUMBER_WIDTH,
            y: inner_area.y,
            width: inner_area.width - LINE_NUMBER_WIDTH,
            height: inner_area.height,
        };

        // Update scroll position to keep cursor visible
        let visible_width = code_area.width as usize;
        let visible_height = code_area.height as usize;
        app.update_scroll(visible_width, visible_height);

        // Get lines from query
        let query_lines: Vec<&str> = if app.query.is_empty() {
            vec![""]
        } else {
            app.query.split('\n').collect()
        };

        // Draw line numbers (with vertical scroll)
        let line_numbers: Vec<Line> = query_lines
            .iter()
            .enumerate()
            .skip(app.query_scroll_y)
            .take(visible_height)
            .map(|(n, _)| {
                Line::from(Span::styled(
                    format!("{:>3} │", n + 1),
                    Style::default().fg(DefaultTheme::COMMENT),
                ))
            })
            .collect();

        let line_num_widget = Paragraph::new(line_numbers);
        f.render_widget(line_num_widget, line_num_area);

        // Get visual selection if in visual mode
        let visual_selection = if app.input_mode == InputMode::Visual {
            Some(app.get_visual_selection())
        } else {
            None
        };

        // Draw syntax-highlighted code with scrolling
        let highlighted_lines = highlight_sql_with_scroll(
            &app.query,
            app.query_scroll_x,
            app.query_scroll_y,
            visible_width,
            visible_height,
            visual_selection,
        );
        let code_widget = Paragraph::new(highlighted_lines);
        f.render_widget(code_widget, code_area);

        // Show cursor when query editor is active
        if active {
            let (cursor_x, cursor_y) = calculate_cursor_position_with_scroll(
                app,
                code_area,
            );
            f.set_cursor(cursor_x, cursor_y);
        }
    }
}

/// Draw the results table panel with tabs
pub fn draw_results_table(f: &mut Frame, app: &mut App, area: Rect, active: bool) {
    let border_style = if active {
        DefaultTheme::active_border()
    } else {
        DefaultTheme::inactive_border()
    };

    // Draw tabs header
    let tabs_area = Rect {
        x: area.x,
        y: area.y,
        width: area.width,
        height: 2,
    };

    let content_area = Rect {
        x: area.x,
        y: area.y + 2,
        width: area.width,
        height: area.height.saturating_sub(2),
    };

    // Draw tab bar
    draw_results_tabs(f, app, tabs_area, active);

    if app.result.columns.is_empty() {
        let help_text = vec![
            Line::from(""),
            Line::from(Span::styled("No results yet", DefaultTheme::dim_text())),
            Line::from(""),
            Line::from(vec![
                Span::styled("Type a query and press ", DefaultTheme::dim_text()),
                Span::styled("Enter", DefaultTheme::info()),
                Span::styled(" to execute", DefaultTheme::dim_text()),
            ]),
        ];
        let empty_msg = Paragraph::new(help_text)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_style(border_style),
            )
            .alignment(Alignment::Center);
        f.render_widget(empty_msg, content_area);
        return;
    }

    // Draw content based on selected tab
    match app.results_tab {
        ResultsTab::Data => draw_results_data(f, app, content_area, active),
        ResultsTab::Columns => draw_results_columns(f, app, content_area, active),
        ResultsTab::Stats => draw_results_stats(f, app, content_area, active),
    }
}

/// Draw the tabs bar
fn draw_results_tabs(f: &mut Frame, app: &App, area: Rect, active: bool) {
    let tabs = vec![
        ("1:Dados", ResultsTab::Data),
        ("2:Colunas", ResultsTab::Columns),
        ("3:Estatísticas", ResultsTab::Stats),
    ];

    let mut spans: Vec<Span> = vec![Span::raw(" ")];
    for (label, tab) in tabs {
        let style = if app.results_tab == tab {
            Style::default()
                .fg(DefaultTheme::TEXT)
                .bg(DefaultTheme::PRIMARY)
                .add_modifier(Modifier::BOLD)
        } else if active {
            Style::default().fg(DefaultTheme::TEXT_DIM)
        } else {
            Style::default().fg(DefaultTheme::TEXT_MUTED)
        };
        spans.push(Span::styled(format!(" {} ", label), style));
        spans.push(Span::raw(" "));
    }

    // Add row/col info on the right
    if !app.result.columns.is_empty() {
        let info = format!(
            "│ {} linhas × {} colunas ",
            app.result.row_count,
            app.result.columns.len()
        );
        spans.push(Span::styled(info, DefaultTheme::dim_text()));
    }

    let tabs_line = Line::from(spans);
    let tabs_widget = Paragraph::new(tabs_line)
        .style(Style::default().bg(DefaultTheme::BG_PANEL));
    f.render_widget(tabs_widget, area);
}

/// Draw the data tab (table rows)
fn draw_results_data(f: &mut Frame, app: &mut App, area: Rect, active: bool) {
    let border_style = if active {
        DefaultTheme::active_border()
    } else {
        DefaultTheme::inactive_border()
    };

    // Build title with stats
    let exec_time_ms = app.result.execution_time.as_secs_f64() * 1000.0;
    let title = format!(
        " Dados │ {} linhas │ {} colunas │ {:.1}ms ",
        app.result.row_count,
        app.result.columns.len(),
        exec_time_ms
    );

    // Calculate available width for columns
    let available_width = area.width.saturating_sub(2) as usize; // minus borders
    let row_num_width = (app.result.rows.len().to_string().len() + 2).max(4) as u16;

    // Calculate which columns to show based on horizontal scroll
    // Each column gets a fixed width for consistent display
    let col_width: u16 = 30; // Fixed column width
    let cols_that_fit = ((available_width as u16).saturating_sub(row_num_width) / col_width).max(1) as usize;

    // Atualiza número de colunas visíveis para uso no handler
    app.results_cols_visible = cols_that_fit;
    
    // Calcula scroll horizontal para manter coluna selecionada visível
    // Se coluna selecionada está antes da área visível, ajusta scroll para esquerda
    if app.results_col_selected < app.results_col_scroll {
        app.results_col_scroll = app.results_col_selected;
    }
    // Se coluna selecionada está depois da área visível, ajusta scroll para direita
    else if app.results_col_selected >= app.results_col_scroll + cols_that_fit {
        app.results_col_scroll = app.results_col_selected.saturating_sub(cols_that_fit - 1);
    }
    
    let col_scroll = app.results_col_scroll;

    // Get visible columns range
    let visible_cols_start = col_scroll;
    let visible_cols_end = (col_scroll + cols_that_fit).min(app.result.columns.len());

    // Build column widths
    let mut widths: Vec<Constraint> = vec![Constraint::Length(row_num_width)];
    for _ in visible_cols_start..visible_cols_end {
        widths.push(Constraint::Length(col_width));
    }

    // Create header row with row number column and type indicators
    let mut header_cells: Vec<Cell> = vec![
        Cell::from(" # ").style(DefaultTheme::table_header())
    ];
    header_cells.extend(
        app.result
            .columns
            .iter()
            .enumerate()
            .skip(visible_cols_start)
            .take(visible_cols_end - visible_cols_start)
            .map(|(i, c)| {
                // Get type indicator
                let type_indicator = get_type_indicator(&c.type_name);
                // Truncate column name to fit
                let name: String = c.name.chars().take(col_width as usize - 4).collect();
                let header_text = format!("{} {}", type_indicator, name);

                let style = if active && i == app.results_col_selected {
                    DefaultTheme::selected()
                } else {
                    DefaultTheme::table_header()
                };
                Cell::from(header_text).style(style)
            })
    );
    let header = Row::new(header_cells).height(1);

    // Create data rows with row numbers
    let visible_height = area.height.saturating_sub(3) as usize;
    let scroll_offset = if app.results_selected >= visible_height {
        app.results_selected.saturating_sub(visible_height - 1)
    } else {
        0
    };

    let rows: Vec<Row> = app
        .result
        .rows
        .iter()
        .enumerate()
        .skip(scroll_offset)
        .take(visible_height)
        .map(|(row_idx, row)| {
            // Row number cell
            let row_num_style = if active && row_idx == app.results_selected {
                DefaultTheme::selected()
            } else {
                DefaultTheme::row_number()
            };
            let mut cells: Vec<Cell> = vec![
                Cell::from(format!("{:>width$} ", row_idx + 1, width = row_num_width as usize - 1))
                    .style(row_num_style)
            ];

            // Data cells - only visible columns
            cells.extend(
                row.iter()
                    .enumerate()
                    .skip(visible_cols_start)
                    .take(visible_cols_end - visible_cols_start)
                    .map(|(col_idx, cell)| {
                        let (value, is_null) = format_cell_value(cell);
                        // Truncate value to fit column
                        let display_value: String = value.chars().take(col_width as usize - 2).collect();

                        let style = if active && row_idx == app.results_selected && col_idx == app.results_col_selected {
                            DefaultTheme::selected()
                        } else if active && row_idx == app.results_selected {
                            DefaultTheme::highlighted()
                        } else if is_null {
                            DefaultTheme::null_value()
                        } else if row_idx % 2 == 1 {
                            DefaultTheme::table_row_alt()
                        } else {
                            DefaultTheme::normal_text()
                        };

                        Cell::from(format!(" {} ", display_value)).style(style)
                    })
            );
            Row::new(cells)
        })
        .collect();

    let table = Table::new(rows, widths)
        .header(header)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(border_style)
                .title(Span::styled(title, DefaultTheme::title())),
        )
        .highlight_style(DefaultTheme::highlighted());

    f.render_widget(table, area);

    // Draw scrollbar if needed
    if app.result.rows.len() > visible_height {
        let scrollbar = Scrollbar::new(ScrollbarOrientation::VerticalRight)
            .begin_symbol(Some("▲"))
            .end_symbol(Some("▼"))
            .track_symbol(Some("│"));

        let mut scrollbar_state = ScrollbarState::new(app.result.rows.len())
            .position(app.results_selected);

        f.render_stateful_widget(
            scrollbar,
            area.inner(&Margin { vertical: 1, horizontal: 0 }),
            &mut scrollbar_state,
        );
    }

    // Draw position indicator at bottom right
    if !app.result.rows.is_empty() {
        let pos_text = format!(
            " Linha {}/{} Coluna {}/{} ",
            app.results_selected + 1,
            app.result.rows.len(),
            app.results_col_selected + 1,
            app.result.columns.len()
        );
        let pos_len = pos_text.len() as u16;
        let pos_x = area.x + area.width.saturating_sub(pos_len + 2);
        let pos_y = area.y + area.height.saturating_sub(1);

        if pos_x > area.x && pos_y < area.y + area.height {
            let pos_span = Span::styled(pos_text, DefaultTheme::dim_text());
            f.render_widget(
                Paragraph::new(pos_span),
                Rect::new(pos_x, pos_y, pos_len, 1),
            );
        }
    }
}

/// Draw the columns tab (column info)
fn draw_results_columns(f: &mut Frame, app: &App, area: Rect, active: bool) {
    let border_style = if active {
        DefaultTheme::active_border()
    } else {
        DefaultTheme::inactive_border()
    };

    let title = format!(" Colunas │ {} total ", app.result.columns.len());

    // Create column info rows - use results_selected for vertical scrolling
    let visible_height = area.height.saturating_sub(3) as usize;
    let scroll_offset = if app.results_selected >= visible_height {
        app.results_selected.saturating_sub(visible_height - 1)
    } else {
        0
    };

    let rows: Vec<Row> = app
        .result
        .columns
        .iter()
        .enumerate()
        .skip(scroll_offset)
        .take(visible_height)
        .map(|(idx, col)| {
            let type_indicator = get_type_indicator(&col.type_name);
            let row_style = if active && idx == app.results_selected {
                DefaultTheme::selected()
            } else if idx % 2 == 1 {
                DefaultTheme::table_row_alt()
            } else {
                DefaultTheme::normal_text()
            };

            Row::new(vec![
                Cell::from(format!(" {:>3} ", idx + 1)).style(DefaultTheme::row_number()),
                Cell::from(format!(" {} ", type_indicator)),
                Cell::from(format!(" {} ", col.name)).style(row_style),
                Cell::from(format!(" {} ", col.type_name)).style(DefaultTheme::dim_text()),
            ])
        })
        .collect();

    let widths = [
        Constraint::Length(6),   // #
        Constraint::Length(4),   // Icon
        Constraint::Min(20),     // Name
        Constraint::Length(20),  // Type
    ];

    let header = Row::new(vec![
        Cell::from(" # ").style(DefaultTheme::table_header()),
        Cell::from(" ").style(DefaultTheme::table_header()),
        Cell::from(" Nome da Coluna ").style(DefaultTheme::table_header()),
        Cell::from(" Tipo de Dados ").style(DefaultTheme::table_header()),
    ])
    .height(1);

    let table = Table::new(rows, widths)
        .header(header)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(border_style)
                .title(Span::styled(title, DefaultTheme::title())),
        );

    f.render_widget(table, area);

    // Draw scrollbar if needed
    if app.result.columns.len() > visible_height {
        let scrollbar = Scrollbar::new(ScrollbarOrientation::VerticalRight)
            .begin_symbol(Some("▲"))
            .end_symbol(Some("▼"))
            .track_symbol(Some("│"));

        let mut scrollbar_state = ScrollbarState::new(app.result.columns.len())
            .position(app.results_selected);

        f.render_stateful_widget(
            scrollbar,
            area.inner(&Margin { vertical: 1, horizontal: 0 }),
            &mut scrollbar_state,
        );
    }
}

/// Draw the stats tab (query statistics)
fn draw_results_stats(f: &mut Frame, app: &App, area: Rect, active: bool) {
    let border_style = if active {
        DefaultTheme::active_border()
    } else {
        DefaultTheme::inactive_border()
    };

    let exec_time = app.result.execution_time;
    let exec_ms = exec_time.as_secs_f64() * 1000.0;

    // Count data types
    let mut type_counts: std::collections::HashMap<String, usize> = std::collections::HashMap::new();
    for col in &app.result.columns {
        *type_counts.entry(col.type_name.clone()).or_insert(0) += 1;
    }

    // Count NULL values
    let mut null_count = 0;
    let mut total_cells = 0;
    for row in &app.result.rows {
        for cell in row {
            total_cells += 1;
            if matches!(cell, CellValue::Null) {
                null_count += 1;
            }
        }
    }

    let null_percentage = if total_cells > 0 {
        (null_count as f64 / total_cells as f64) * 100.0
    } else {
        0.0
    };

    // Build stats text
    let mut stats_lines: Vec<Line> = vec![
        Line::from(""),
        Line::from(Span::styled("═══ ESTATÍSTICAS DA QUERY ═══", DefaultTheme::info())),
        Line::from(""),
        Line::from(vec![
            Span::styled("  Tempo de Execução:  ", DefaultTheme::dim_text()),
            Span::styled(format!("{:.2} ms", exec_ms), DefaultTheme::success()),
        ]),
        Line::from(vec![
            Span::styled("  Linhas Retornadas:   ", DefaultTheme::dim_text()),
            Span::styled(format_number(app.result.row_count as i64), DefaultTheme::info()),
        ]),
        Line::from(vec![
            Span::styled("  Colunas:         ", DefaultTheme::dim_text()),
            Span::styled(format!("{}", app.result.columns.len()), DefaultTheme::info()),
        ]),
        Line::from(vec![
            Span::styled("  Total de Células:     ", DefaultTheme::dim_text()),
            Span::styled(format_number(total_cells as i64), DefaultTheme::normal_text()),
        ]),
        Line::from(vec![
            Span::styled("  Valores NULL:     ", DefaultTheme::dim_text()),
            Span::styled(format!("{} ({:.1}%)", format_number(null_count as i64), null_percentage), DefaultTheme::warning()),
        ]),
        Line::from(""),
        Line::from(Span::styled("═══ TIPOS DE DADOS ═══", DefaultTheme::info())),
        Line::from(""),
    ];

    // Add type breakdown
    let mut type_vec: Vec<(&String, &usize)> = type_counts.iter().collect();
    type_vec.sort_by(|a, b| b.1.cmp(a.1));

    for (type_name, count) in type_vec.iter().take(10) {
        let indicator = get_type_indicator(type_name);
        stats_lines.push(Line::from(vec![
            Span::styled(format!("  {} ", indicator), DefaultTheme::normal_text()),
            Span::styled(format!("{:<20}", type_name), DefaultTheme::dim_text()),
            Span::styled(format!("{:>5} coluna(s)", count), DefaultTheme::normal_text()),
        ]));
    }

    stats_lines.push(Line::from(""));
    stats_lines.push(Line::from(Span::styled("═══ ATALHOS ═══", DefaultTheme::info())));
    stats_lines.push(Line::from(""));
    stats_lines.push(Line::from(vec![
        Span::styled("  Ctrl+E  ", DefaultTheme::info()),
        Span::styled("Exportar para CSV", DefaultTheme::dim_text()),
    ]));
    stats_lines.push(Line::from(vec![
        Span::styled("  Ctrl+S  ", DefaultTheme::info()),
        Span::styled("Exportar para JSON", DefaultTheme::dim_text()),
    ]));
    stats_lines.push(Line::from(vec![
        Span::styled("  Ctrl+I  ", DefaultTheme::info()),
        Span::styled("Copiar linha como INSERT", DefaultTheme::dim_text()),
    ]));
    stats_lines.push(Line::from(vec![
        Span::styled("  Ctrl+Y  ", DefaultTheme::info()),
        Span::styled("Copiar valor da célula", DefaultTheme::dim_text()),
    ]));

    let stats_widget = Paragraph::new(stats_lines)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(border_style)
                .title(Span::styled(" Estatísticas ", DefaultTheme::title())),
        );

    f.render_widget(stats_widget, area);
}

/// Get type indicator emoji for column type
fn get_type_indicator(type_name: &str) -> &'static str {
    match type_name.to_uppercase().as_str() {
        "INT" | "INTEGER" | "BIGINT" | "SMALLINT" | "TINYINT" => "🔢",
        "DECIMAL" | "NUMERIC" | "FLOAT" | "REAL" | "MONEY" | "SMALLMONEY" => "💰",
        "VARCHAR" | "NVARCHAR" | "CHAR" | "NCHAR" | "TEXT" | "NTEXT" | "VARCHAR(MAX)" => "📝",
        "DATETIME" | "DATETIME2" | "DATE" | "TIME" | "DATETIMEOFFSET" | "SMALLDATETIME" => "📅",
        "BIT" => "✓",
        "BINARY" | "VARBINARY" | "VARBINARY(MAX)" | "IMAGE" => "📦",
        "UNIQUEIDENTIFIER" => "🔑",
        "XML" => "📄",
        _ => "•",
    }
}

/// Format cell value for display with NULL handling
fn format_cell_value(cell: &CellValue) -> (String, bool) {
    match cell {
        CellValue::Null => ("NULL".to_string(), true),
        CellValue::Bool(v) => (if *v { "✓ true" } else { "✗ false" }.to_string(), false),
        CellValue::Int(v) => (format_number(*v), false),
        CellValue::Float(v) => (format!("{:.4}", v), false),
        CellValue::String(v) => {
            // Truncate long strings
            if v.len() > 50 {
                (format!("{}…", &v[..47]), false)
            } else {
                (v.clone(), false)
            }
        }
        CellValue::DateTime(v) => (v.clone(), false),
        CellValue::Binary(v) => (format!("0x{}…", &hex_encode(&v[..v.len().min(8)])), false),
    }
}

/// Format number with thousand separators
fn format_number(n: i64) -> String {
    let s = n.abs().to_string();
    let mut result = String::new();
    for (i, c) in s.chars().rev().enumerate() {
        if i > 0 && i % 3 == 0 {
            result.push(',');
        }
        result.push(c);
    }
    if n < 0 {
        result.push('-');
    }
    result.chars().rev().collect()
}

/// Hex encode bytes
fn hex_encode(data: &[u8]) -> String {
    data.iter().map(|b| format!("{:02X}", b)).collect()
}

/// Draw the schema explorer panel
pub fn draw_schema_explorer(f: &mut Frame, app: &App, area: Rect, active: bool) {
    let border_style = if active {
        DefaultTheme::active_border()
    } else {
        DefaultTheme::inactive_border()
    };

    // Título com indicador de busca ativa
    let title = if !app.schema_search_query.is_empty() {
        format!(" Schema [s] 🔍 {} ", app.schema_search_query)
    } else if active {
        " Schema [s] ▪ ".to_string()
    } else {
        " Schema [s] ".to_string()
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

/// Draw the history panel
pub fn draw_history_panel(f: &mut Frame, app: &App, area: Rect, active: bool) {
    let border_style = if active {
        DefaultTheme::active_border()
    } else {
        DefaultTheme::inactive_border()
    };

    let title = if active { " Histórico [4] ▪ " } else { " Histórico [4] " };

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

    f.render_widget(list, area);
}

/// SQL syntax highlighting
fn highlight_sql(sql: &str) -> Vec<Line<'static>> {
    let keywords = [
        "SELECT", "FROM", "WHERE", "AND", "OR", "NOT", "IN", "LIKE", "BETWEEN",
        "ORDER", "BY", "ASC", "DESC", "GROUP", "HAVING", "JOIN", "INNER", "LEFT",
        "RIGHT", "OUTER", "FULL", "CROSS", "ON", "AS", "DISTINCT", "TOP", "WITH",
        "INSERT", "INTO", "VALUES", "UPDATE", "SET", "DELETE", "CREATE", "TABLE",
        "ALTER", "DROP", "INDEX", "VIEW", "PROCEDURE", "FUNCTION", "TRIGGER",
        "BEGIN", "END", "IF", "ELSE", "WHILE", "RETURN", "DECLARE", "EXEC", "EXECUTE",
        "NULL", "IS", "CASE", "WHEN", "THEN", "UNION", "ALL", "EXISTS", "COUNT",
        "SUM", "AVG", "MIN", "MAX", "CAST", "CONVERT", "COALESCE", "ISNULL",
    ];

    let mut lines: Vec<Line> = Vec::new();

    for line in sql.lines() {
        let mut spans: Vec<Span> = Vec::new();
        let mut current_word = String::new();
        let mut in_string = false;
        let mut string_char = ' ';
        let mut in_comment = false;

        let chars: Vec<char> = line.chars().collect();
        let mut i = 0;

        while i < chars.len() {
            let c = chars[i];

            // Check for line comment
            if !in_string && i + 1 < chars.len() && chars[i] == '-' && chars[i + 1] == '-' {
                if !current_word.is_empty() {
                    spans.push(colorize_word(&current_word, &keywords));
                    current_word.clear();
                }
                // Rest of line is comment
                let comment: String = chars[i..].iter().collect();
                spans.push(Span::styled(comment, Style::default().fg(DefaultTheme::COMMENT)));
                break;
            }

            // Handle strings
            if (c == '\'' || c == '"') && !in_comment {
                if in_string && c == string_char {
                    current_word.push(c);
                    spans.push(Span::styled(
                        current_word.clone(),
                        Style::default().fg(DefaultTheme::STRING),
                    ));
                    current_word.clear();
                    in_string = false;
                } else if !in_string {
                    if !current_word.is_empty() {
                        spans.push(colorize_word(&current_word, &keywords));
                        current_word.clear();
                    }
                    in_string = true;
                    string_char = c;
                    current_word.push(c);
                } else {
                    current_word.push(c);
                }
            } else if in_string {
                current_word.push(c);
            } else if c.is_whitespace() || "(),;.=<>+-*/".contains(c) {
                if !current_word.is_empty() {
                    spans.push(colorize_word(&current_word, &keywords));
                    current_word.clear();
                }
                spans.push(Span::styled(
                    c.to_string(),
                    Style::default().fg(DefaultTheme::OPERATOR),
                ));
            } else {
                current_word.push(c);
            }

            i += 1;
        }

        if !current_word.is_empty() {
            spans.push(colorize_word(&current_word, &keywords));
        }

        lines.push(Line::from(spans));
    }

    lines
}

fn colorize_word(word: &str, keywords: &[&str]) -> Span<'static> {
    let upper = word.to_uppercase();

    if keywords.contains(&upper.as_str()) {
        Span::styled(
            word.to_string(),
            Style::default()
                .fg(DefaultTheme::KEYWORD)
                .add_modifier(Modifier::BOLD),
        )
    } else if word.chars().all(|c| c.is_ascii_digit() || c == '.') {
        Span::styled(
            word.to_string(),
            Style::default().fg(DefaultTheme::NUMBER),
        )
    } else if word.starts_with('@') || word.starts_with("@@") {
        Span::styled(
            word.to_string(),
            Style::default().fg(DefaultTheme::FUNCTION),
        )
    } else {
        Span::styled(word.to_string(), DefaultTheme::normal_text())
    }
}

/// Calculate cursor position with scroll offset
fn calculate_cursor_position_with_scroll(app: &App, code_area: Rect) -> (u16, u16) {
    let (line, col) = app.get_cursor_line_col();

    // Adjust for scroll offset
    let visible_line = line.saturating_sub(app.query_scroll_y);
    let visible_col = col.saturating_sub(app.query_scroll_x);

    let x = (code_area.x + visible_col as u16).min(code_area.x + code_area.width.saturating_sub(1));
    let y = (code_area.y + visible_line as u16).min(code_area.y + code_area.height.saturating_sub(1));

    (x, y)
}

/// SQL syntax highlighting with scroll support and visual selection
fn highlight_sql_with_scroll(
    sql: &str,
    scroll_x: usize,
    scroll_y: usize,
    visible_width: usize,
    visible_height: usize,
    visual_selection: Option<(usize, usize)>, // (start, end) char positions
) -> Vec<Line<'static>> {
    let keywords = [
        "SELECT", "FROM", "WHERE", "AND", "OR", "NOT", "IN", "LIKE", "BETWEEN",
        "ORDER", "BY", "ASC", "DESC", "GROUP", "HAVING", "JOIN", "INNER", "LEFT",
        "RIGHT", "OUTER", "FULL", "CROSS", "ON", "AS", "DISTINCT", "TOP", "WITH",
        "INSERT", "INTO", "VALUES", "UPDATE", "SET", "DELETE", "CREATE", "TABLE",
        "ALTER", "DROP", "INDEX", "VIEW", "PROCEDURE", "FUNCTION", "TRIGGER",
        "BEGIN", "END", "IF", "ELSE", "WHILE", "RETURN", "DECLARE", "EXEC", "EXECUTE",
        "NULL", "IS", "CASE", "WHEN", "THEN", "UNION", "ALL", "EXISTS", "COUNT",
        "SUM", "AVG", "MIN", "MAX", "CAST", "CONVERT", "COALESCE", "ISNULL",
    ];

    // Visual selection style (inverted colors)
    let visual_style = Style::default()
        .fg(DefaultTheme::BG_DARK)
        .bg(DefaultTheme::PRIMARY);

    let source_lines: Vec<&str> = sql.split('\n').collect();
    let mut lines: Vec<Line> = Vec::new();

    // Calculate absolute character position at the start of each line
    let mut line_starts: Vec<usize> = vec![0];
    let mut pos = 0;
    for line in &source_lines {
        pos += line.len() + 1; // +1 for newline
        line_starts.push(pos);
    }

    for (line_idx, line_content) in source_lines.iter().enumerate().skip(scroll_y).take(visible_height) {
        let line_start_pos = line_starts[line_idx];
        
        // Apply horizontal scroll
        let display_content: String = line_content
            .chars()
            .skip(scroll_x)
            .take(visible_width)
            .collect();

        let mut spans: Vec<Span> = Vec::new();
        let mut current_word = String::new();
        let mut in_string = false;
        let mut string_char = ' ';

        let chars: Vec<char> = display_content.chars().collect();
        let mut i = 0;

        while i < chars.len() {
            let c = chars[i];
            // Calculate absolute position in the original string
            let abs_pos = line_start_pos + scroll_x + i;
            
            // Check if this character is in visual selection
            let in_visual = visual_selection.map_or(false, |(start, end)| {
                abs_pos >= start && abs_pos <= end
            });

            // Check for line comment
            if !in_string && i + 1 < chars.len() && chars[i] == '-' && chars[i + 1] == '-' {
                if !current_word.is_empty() {
                    spans.push(colorize_word(&current_word, &keywords));
                    current_word.clear();
                }
                // Rest of line is comment - check if any part is in selection
                let comment: String = chars[i..].iter().collect();
                if in_visual {
                    // Handle comment with visual selection
                    for (j, ch) in comment.chars().enumerate() {
                        let ch_abs_pos = line_start_pos + scroll_x + i + j;
                        let ch_in_visual = visual_selection.map_or(false, |(start, end)| {
                            ch_abs_pos >= start && ch_abs_pos <= end
                        });
                        if ch_in_visual {
                            spans.push(Span::styled(ch.to_string(), visual_style));
                        } else {
                            spans.push(Span::styled(ch.to_string(), Style::default().fg(DefaultTheme::COMMENT)));
                        }
                    }
                } else {
                    spans.push(Span::styled(comment, Style::default().fg(DefaultTheme::COMMENT)));
                }
                break;
            }

            // If in visual selection, use visual style
            if in_visual {
                if !current_word.is_empty() {
                    spans.push(colorize_word(&current_word, &keywords));
                    current_word.clear();
                }
                spans.push(Span::styled(c.to_string(), visual_style));
                i += 1;
                continue;
            }

            // Handle strings
            if (c == '\'' || c == '"') && !in_string {
                if !current_word.is_empty() {
                    spans.push(colorize_word(&current_word, &keywords));
                    current_word.clear();
                }
                in_string = true;
                string_char = c;
                current_word.push(c);
            } else if in_string && c == string_char {
                current_word.push(c);
                spans.push(Span::styled(
                    current_word.clone(),
                    Style::default().fg(DefaultTheme::STRING),
                ));
                current_word.clear();
                in_string = false;
            } else if in_string {
                current_word.push(c);
            } else if c.is_whitespace() || "(),;.=<>+-*/[]".contains(c) {
                if !current_word.is_empty() {
                    spans.push(colorize_word(&current_word, &keywords));
                    current_word.clear();
                }
                spans.push(Span::styled(
                    c.to_string(),
                    Style::default().fg(DefaultTheme::OPERATOR),
                ));
            } else {
                current_word.push(c);
            }

            i += 1;
        }

        if !current_word.is_empty() {
            if in_string {
                spans.push(Span::styled(current_word, Style::default().fg(DefaultTheme::STRING)));
            } else {
                spans.push(colorize_word(&current_word, &keywords));
            }
        }

        lines.push(Line::from(spans));
    }

    // Pad with empty lines if needed
    while lines.len() < visible_height {
        lines.push(Line::from(""));
    }

    lines
}
