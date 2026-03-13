use bytesize::ByteSize;
use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, Paragraph, Wrap};
use ratatui::Frame;

use crate::app::App;
use crate::ui::style::UiStyle;
use crate::utils::format_size;

pub fn draw_help(f: &mut Frame, app: &App, style: &UiStyle) {
    let area = centered_rect(60, 70, f.area());
    f.render_widget(Clear, area);

    let help_text = vec![
        Line::from(Span::styled(
            " Keyboard Shortcuts ",
            Style::default()
                .fg(Color::White)
                .add_modifier(Modifier::BOLD),
        )),
        Line::from(""),
        Line::from(vec![
            Span::styled("  F10         ", Style::default().fg(style.fg_accent)),
            Span::raw("Open menu bar"),
        ]),
        Line::from(vec![
            Span::styled("  q / Esc     ", Style::default().fg(style.fg_accent)),
            Span::raw("Quit"),
        ]),
        Line::from(vec![
            Span::styled("  1 / 2 / 3   ", Style::default().fg(style.fg_accent)),
            Span::raw("Switch tabs"),
        ]),
        Line::from(vec![
            Span::styled("  Tab         ", Style::default().fg(style.fg_accent)),
            Span::raw("Switch pane (Tree <-> Map)"),
        ]),
        Line::from(vec![
            Span::styled("  Up/Down     ", Style::default().fg(style.fg_accent)),
            Span::raw("Navigate tree"),
        ]),
        Line::from(vec![
            Span::styled("  Left        ", Style::default().fg(style.fg_accent)),
            Span::raw("Collapse / go to parent"),
        ]),
        Line::from(vec![
            Span::styled("  Right       ", Style::default().fg(style.fg_accent)),
            Span::raw("Expand directory"),
        ]),
        Line::from(vec![
            Span::styled("  Enter       ", Style::default().fg(style.fg_accent)),
            Span::raw("Zoom into directory (treemap)"),
        ]),
        Line::from(vec![
            Span::styled("  Backspace   ", Style::default().fg(style.fg_accent)),
            Span::raw("Zoom out (treemap)"),
        ]),
        Line::from(vec![
            Span::styled("  d / Del     ", Style::default().fg(style.fg_accent)),
            Span::raw("Delete selected file/dir"),
        ]),
        Line::from(vec![
            Span::styled("  o           ", Style::default().fg(style.fg_accent)),
            Span::raw("Open in Finder"),
        ]),
        Line::from(vec![
            Span::styled("  c           ", Style::default().fg(style.fg_accent)),
            Span::raw("Copy path to clipboard"),
        ]),
        Line::from(vec![
            Span::styled("  p           ", Style::default().fg(style.fg_accent)),
            Span::raw("Change directory"),
        ]),
        Line::from(vec![
            Span::styled("  r           ", Style::default().fg(style.fg_accent)),
            Span::raw("Refresh scan"),
        ]),
        Line::from(vec![
            Span::styled("  R           ", Style::default().fg(style.fg_accent)),
            Span::raw("Rescan selected subtree"),
        ]),
        Line::from(vec![
            Span::styled("  /           ", Style::default().fg(style.fg_accent)),
            Span::raw("Search by name"),
        ]),
        Line::from(vec![
            Span::styled("  n / N       ", Style::default().fg(style.fg_accent)),
            Span::raw("Next / previous match"),
        ]),
        Line::from(vec![
            Span::styled("  e           ", Style::default().fg(style.fg_accent)),
            Span::raw("Export CSV"),
        ]),
        Line::from(vec![
            Span::styled("  F           ", Style::default().fg(style.fg_accent)),
            Span::raw("Filter by size (e.g., 10M, 1.5G)"),
        ]),
        Line::from(vec![
            Span::styled("  C           ", Style::default().fg(style.fg_accent)),
            Span::raw("Clear size filter"),
        ]),
        Line::from(vec![
            Span::styled("  ?           ", Style::default().fg(style.fg_accent)),
            Span::raw("Toggle this help"),
        ]),
        Line::from(""),
        Line::from(Span::styled(
            format!("  {}", app.strings.press_any_key),
            Style::default().fg(Color::DarkGray),
        )),
    ];

    let title = format!(" {} ", app.strings.help_title);
    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(style.border_type)
        .title(title)
        .border_style(Style::default().fg(style.fg_accent));

    let paragraph = Paragraph::new(help_text)
        .block(block)
        .wrap(Wrap { trim: false });
    f.render_widget(paragraph, area);
}

pub fn draw_confirm_delete(f: &mut Frame, app: &App, style: &UiStyle) {
    if let Some((path, size, _node_id)) = &app.confirm_delete {
        let area = centered_rect(50, 25, f.area());
        f.render_widget(Clear, area);

        let size_str = format_size(*size);

        let text = vec![
            Line::from(""),
            Line::from(Span::styled(
                format!(" {} ", app.strings.are_you_sure_delete),
                Style::default().fg(Color::Red).add_modifier(Modifier::BOLD),
            )),
            Line::from(""),
            Line::from(Span::styled(
                format!(" {} ", path.display()),
                Style::default().fg(Color::Yellow),
            )),
            Line::from(Span::styled(
                format!(" {}: {} ", app.strings.size, size_str),
                Style::default().fg(Color::Cyan),
            )),
            Line::from(""),
            Line::from(vec![
                Span::styled(
                    format!("  {} ", app.strings.confirm_yes),
                    Style::default()
                        .fg(Color::Green)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::raw(format!("{}   ", app.strings.confirm)),
                Span::styled(
                    format!("  {} ", app.strings.confirm_no),
                    Style::default().fg(Color::Red).add_modifier(Modifier::BOLD),
                ),
                Span::raw(app.strings.cancel),
            ]),
        ];

        let title = format!(" {} ", app.strings.confirm_delete);
        let block = Block::default()
            .borders(Borders::ALL)
            .border_type(style.border_type)
            .title(title)
            .border_style(Style::default().fg(Color::Red));

        let paragraph = Paragraph::new(text).block(block).wrap(Wrap { trim: false });
        f.render_widget(paragraph, area);
    }
}

pub fn draw_duplicates(f: &mut Frame, app: &App, area: Rect, style: &UiStyle) {
    let title = format!(" {} ", app.strings.duplicates);
    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(style.border_type)
        .title(title)
        .border_style(Style::default().fg(style.border_color));

    if app.dupes_state == crate::app::DupeState::Scanning {
        let text = Paragraph::new(app.strings.scanning_duplicates).block(block);
        f.render_widget(text, area);
        return;
    }

    if app.duplicates.is_empty() {
        let text = Paragraph::new(vec![
            Line::from(""),
            Line::from(format!("  {}", app.strings.no_duplicates_press_s)),
        ])
        .block(block);
        f.render_widget(text, area);
        return;
    }

    let total_wasted: u64 = app.duplicates.iter().map(|d| d.wasted_size()).sum();
    let title = format!(
        " {} ",
        app.strings
            .duplicate_groups_wasted
            .replace("{}", &app.duplicates.len().to_string())
            .replace("{}", &format!("{}", ByteSize(total_wasted)))
    );

    let block = block.title(title);
    let inner = block.inner(area);
    f.render_widget(block, area);

    let mut lines: Vec<Line> = Vec::new();

    for (i, group) in app.duplicates.iter().enumerate() {
        let is_selected = i == app.dupes_selected_index;
        let header_style = if is_selected {
            Style::default()
                .fg(style.selected_fg)
                .bg(style.selected_bg)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD)
        };

        lines.push(Line::from(Span::styled(
            format!(
                "  {} x {} (wasted: {})",
                group.paths.len(),
                ByteSize(group.size),
                ByteSize(group.wasted_size())
            ),
            header_style,
        )));

        for path in &group.paths {
            lines.push(Line::from(Span::styled(
                format!("    {}", path.display()),
                Style::default().fg(Color::Gray),
            )));
        }

        lines.push(Line::from(""));

        if lines.len() > inner.height as usize {
            break;
        }
    }

    let paragraph = Paragraph::new(lines);
    f.render_widget(paragraph, inner);
}

pub fn draw_path_input(f: &mut Frame, app: &App, style: &UiStyle) {
    if let Some(input) = &app.path_input {
        let area = centered_rect(70, 0, f.area());
        let area = Rect::new(area.x, f.area().height / 2 - 4, area.width, 8);
        f.render_widget(Clear, area);

        let is_valid = input.validate().is_some();
        let border_color = if is_valid {
            Color::Green
        } else {
            Color::Yellow
        };

        let block = Block::default()
            .borders(Borders::ALL)
            .border_type(style.border_type)
            .title(" Change Directory (Enter=confirm, Esc=cancel, Tab=complete) ")
            .border_style(Style::default().fg(border_color));

        let inner = block.inner(area);
        f.render_widget(block, area);

        let before_cursor = &input.input[..input.cursor];
        let at_cursor = if input.cursor < input.input.len() {
            &input.input[input.cursor..input.cursor + 1]
        } else {
            " "
        };
        let after_cursor = if input.cursor < input.input.len() {
            &input.input[input.cursor + 1..]
        } else {
            ""
        };

        let input_line = Line::from(vec![
            Span::styled(
                "  Path: ",
                Style::default()
                    .fg(style.fg_accent)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(before_cursor, Style::default().fg(Color::White)),
            Span::styled(
                at_cursor,
                Style::default().fg(Color::Black).bg(Color::White),
            ),
            Span::styled(after_cursor, Style::default().fg(Color::White)),
        ]);

        let status_line = if is_valid {
            Line::from(Span::styled(
                "  Valid directory",
                Style::default().fg(Color::Green),
            ))
        } else {
            Line::from(Span::styled(
                "  Not a valid directory",
                Style::default().fg(Color::Red),
            ))
        };

        let completions_line = if let Some(idx) = input.completion_index {
            let total = input.completions.len();
            Line::from(Span::styled(
                format!("  Tab: completion {}/{}", idx + 1, total),
                Style::default().fg(Color::DarkGray),
            ))
        } else {
            Line::from(Span::styled(
                "  Tab: autocomplete path",
                Style::default().fg(Color::DarkGray),
            ))
        };

        let text = vec![
            Line::from(""),
            input_line,
            Line::from(""),
            status_line,
            completions_line,
        ];

        let paragraph = Paragraph::new(text);
        f.render_widget(paragraph, inner);
    }
}

pub fn draw_search_input(f: &mut Frame, app: &App) {
    if let Some(input) = &app.search_input {
        let area = f.area();
        let y = area.height.saturating_sub(1);
        let bar_area = Rect::new(area.x, y, area.width, 1);

        let cursor_char = if input.is_empty() { " " } else { "" };
        let line = Line::from(vec![
            Span::styled(
                "/",
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(input.as_str(), Style::default().fg(Color::White)),
            Span::styled(
                cursor_char,
                Style::default().fg(Color::Black).bg(Color::White),
            ),
        ]);

        let bar = Paragraph::new(line).style(Style::default().bg(Color::Rgb(20, 20, 20)));
        f.render_widget(bar, bar_area);
    }
}

pub fn draw_filter_input(f: &mut Frame, app: &App) {
    if let Some(input) = &app.filter_input {
        let area = f.area();
        let y = area.height.saturating_sub(1);
        let bar_area = Rect::new(area.x, y, area.width, 1);

        let cursor_char = if input.is_empty() { " " } else { "" };
        let prompt = format!("{}: ", app.strings.min_size);
        let line = Line::from(vec![
            Span::styled(
                &prompt,
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(input.as_str(), Style::default().fg(Color::White)),
            Span::styled(
                cursor_char,
                Style::default().fg(Color::Black).bg(Color::White),
            ),
            Span::styled(
                " (e.g., 10M, 1.5G, 500K, Enter=apply, Esc=cancel)",
                Style::default().fg(Color::DarkGray),
            ),
        ]);

        let bar = Paragraph::new(line).style(Style::default().bg(Color::Rgb(20, 20, 20)));
        f.render_widget(bar, bar_area);
    }
}

pub fn draw_top_files(f: &mut Frame, app: &App, style: &UiStyle) {
    let area = centered_rect(80, 80, f.area());
    f.render_widget(Clear, area);

    let title = format!(" {} ({}) ", app.strings.top_files, app.top_files_count);
    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(style.border_type)
        .title(title)
        .border_style(Style::default().fg(style.fg_accent));

    let inner = block.inner(area);
    f.render_widget(block, area);

    if app.top_files.is_empty() {
        let text = Paragraph::new("No files found");
        f.render_widget(text, inner);
        return;
    }

    let mut lines: Vec<Line> = Vec::new();
    let tree = match &app.tree {
        Some(t) => t,
        None => {
            let text = Paragraph::new("No tree data");
            f.render_widget(text, inner);
            return;
        }
    };

    for (rank, (node_id, size)) in app.top_files.iter().enumerate() {
        let is_selected = rank == app.top_files_selected;
        let path = tree.full_path(*node_id);

        let size_str = format_size(*size);
        let rank_str = format!("{:3}. ", rank + 1);

        let path_str = path.to_string_lossy();
        let max_path_len = inner.width.saturating_sub(20) as usize;
        let truncated_path = if path_str.len() > max_path_len {
            format!(
                "...{}",
                &path_str[path_str.len().saturating_sub(max_path_len - 3)..]
            )
        } else {
            path_str.to_string()
        };

        let line_style = if is_selected {
            Style::default()
                .fg(style.selected_fg)
                .bg(style.selected_bg)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default()
        };

        lines.push(Line::from(vec![
            Span::styled(rank_str, line_style),
            Span::styled(
                format!("{:>8} ", size_str),
                Style::default().fg(style.fg_accent),
            ),
            Span::styled(truncated_path, line_style),
        ]));
    }

    let paragraph = Paragraph::new(lines);
    f.render_widget(paragraph, inner);
}

/// Create a centered rectangle.
fn centered_rect(percent_x: u16, percent_y: u16, r: Rect) -> Rect {
    let popup_width = r.width * percent_x / 100;
    let popup_height = r.height * percent_y / 100;
    let x = r.x + (r.width - popup_width) / 2;
    let y = r.y + (r.height - popup_height) / 2;
    Rect::new(x, y, popup_width, popup_height)
}
