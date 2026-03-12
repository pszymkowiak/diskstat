use bytesize::ByteSize;
use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph, Row, Table};
use ratatui::layout::Constraint;

use crate::app::App;
use crate::ui::style::UiStyle;

/// Draw the full extensions tab view.
pub fn draw(f: &mut Frame, app: &App, area: Rect, style: &UiStyle) {
    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(style.border_type)
        .title(" Extensions ")
        .border_style(Style::default().fg(style.border_color));

    let total_size = app.total_size.max(1) as f64;

    let header = Row::new(vec!["Extension", "Size", "%", "Files", "Bar"])
        .style(
            Style::default()
                .fg(Color::White)
                .add_modifier(Modifier::BOLD),
        )
        .bottom_margin(1);

    let rows: Vec<Row> = app
        .ext_stats
        .iter()
        .enumerate()
        .map(|(i, stat)| {
            let pct = stat.total_size as f64 / total_size * 100.0;
            let bar_width = 20;
            let filled = ((pct / 100.0) * bar_width as f64) as usize;
            let bar = format!(
                "{}{}",
                style.bar_filled.repeat(filled),
                style.bar_empty.repeat(bar_width - filled)
            );

            let row_style = if i == app.ext_selected_index {
                Style::default()
                    .bg(style.selected_bg)
                    .fg(style.selected_fg)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(stat.color)
            };

            Row::new(vec![
                format!(".{}", stat.extension),
                format!("{}", ByteSize(stat.total_size)),
                format!("{:.1}%", pct),
                format!("{}", stat.file_count),
                bar,
            ])
            .style(row_style)
        })
        .collect();

    let widths = [
        Constraint::Length(12),
        Constraint::Length(10),
        Constraint::Length(7),
        Constraint::Length(8),
        Constraint::Min(20),
    ];

    let table = Table::new(rows, widths)
        .header(header)
        .block(block);

    f.render_widget(table, area);
}

/// Draw a compact extension summary for the bottom panel in Tree+Map view.
pub fn draw_summary(f: &mut Frame, app: &App, area: Rect, style: &UiStyle) {
    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(style.border_type)
        .title(" Extensions ")
        .border_style(Style::default().fg(style.border_color));

    let inner = block.inner(area);
    f.render_widget(block, area);

    let total_size = app.total_size.max(1) as f64;
    let max_items = inner.height as usize;
    let col_width = 30usize;
    let cols = (inner.width as usize / col_width).max(1);

    let mut lines: Vec<Line> = Vec::new();

    for row in 0..max_items {
        let mut spans: Vec<Span> = Vec::new();

        for col in 0..cols {
            let idx = col * max_items + row;
            if idx >= app.ext_stats.len() {
                break;
            }

            let stat = &app.ext_stats[idx];
            let pct = stat.total_size as f64 / total_size * 100.0;

            spans.push(Span::styled(
                style.ext_pastille,
                Style::default().fg(stat.color),
            ));
            spans.push(Span::styled(
                format!("{:<8} {:>8} {:>5.1}%  ",
                    format!(".{}", stat.extension),
                    format!("{}", ByteSize(stat.total_size)),
                    pct
                ),
                Style::default().fg(Color::Gray),
            ));
        }

        if !spans.is_empty() {
            lines.push(Line::from(spans));
        }
    }

    let paragraph = Paragraph::new(lines);
    f.render_widget(paragraph, inner);
}
