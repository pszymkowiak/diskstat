use bytesize::ByteSize;
use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;
use ratatui::Frame;

use crate::app::{ActiveTab, App, ScanState};
use crate::ui::style::UiStyle;

pub fn draw(f: &mut Frame, app: &App, area: Rect, style: &UiStyle) {
    let mut spans: Vec<Span> = Vec::new();

    match app.scan_state {
        ScanState::Scanning => {
            spans.push(Span::styled(
                format!(" {} ", app.strings.scanning.to_uppercase()),
                Style::default()
                    .fg(Color::Black)
                    .bg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            ));

            // Progress bar: show file count and size
            let progress_text = format!(
                " {} {} ({}) ",
                app.file_count,
                app.strings.files,
                ByteSize(app.total_size)
            );
            spans.push(Span::raw(progress_text));

            // Add animated progress indicator
            let frame = (std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_millis()
                / 200)
                % 8;
            let progress_chars = ["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧"];
            spans.push(Span::styled(
                progress_chars[frame as usize],
                Style::default().fg(Color::Yellow),
            ));
        }
        ScanState::Done => {
            spans.push(Span::styled(
                format!(" {} ", app.strings.done),
                Style::default()
                    .fg(Color::Black)
                    .bg(Color::Green)
                    .add_modifier(Modifier::BOLD),
            ));
            spans.push(Span::raw(format!(
                " {} {} | {} ",
                app.file_count,
                app.strings.files,
                ByteSize(app.total_size)
            )));
        }
        ScanState::Idle => {
            spans.push(Span::styled(
                format!(" {} ", app.strings.idle),
                Style::default().fg(Color::Black).bg(Color::Gray),
            ));
        }
    }

    // Disk space info
    if app.disk_total > 0 {
        let used = app.disk_total.saturating_sub(app.disk_free);
        let pct = if app.disk_total > 0 {
            (used as f64 / app.disk_total as f64 * 100.0) as u64
        } else {
            0
        };
        spans.push(Span::styled(" | ", Style::default().fg(Color::DarkGray)));
        spans.push(Span::styled(
            format!(
                "{}: {} {} / {} ({}% {})",
                app.strings.disk,
                ByteSize(app.disk_free),
                app.strings.free,
                ByteSize(app.disk_total),
                pct,
                app.strings.used
            ),
            Style::default().fg(Color::Rgb(140, 140, 140)),
        ));
    }

    // Sort mode indicator (only show in TreeMap tab)
    if app.active_tab == ActiveTab::TreeMap {
        spans.push(Span::styled(" | ", Style::default().fg(Color::DarkGray)));
        spans.push(Span::styled(
            format!("Sort: {}", app.sort_mode.display_name()),
            Style::default().fg(Color::Cyan),
        ));
    }

    // Selected item info
    if let Some(selected) = app.tree_state.selected {
        if let Some(tree) = &app.tree {
            let entry = tree.arena[selected].get();
            let path = tree.full_path(selected);
            spans.push(Span::styled(" | ", Style::default().fg(Color::DarkGray)));
            spans.push(Span::styled(
                format!("{}", path.display()),
                Style::default().fg(style.fg_accent),
            ));
            spans.push(Span::raw(format!(" ({})", ByteSize(entry.size))));
        }
    }

    // Status message
    if let Some(msg) = &app.status_message {
        spans.push(Span::styled(" | ", Style::default().fg(Color::DarkGray)));
        spans.push(Span::styled(
            msg.clone(),
            Style::default().fg(Color::Yellow),
        ));
    }

    // Right-aligned help hint
    let hint = " F10:menu t:treemap p:chdir r:rescan q:quit ?:help ";
    let used: usize = spans.iter().map(|s| s.content.len()).sum();
    let remaining = (area.width as usize).saturating_sub(used + hint.len());
    spans.push(Span::raw(" ".repeat(remaining)));
    spans.push(Span::styled(hint, Style::default().fg(Color::DarkGray)));

    let line = Line::from(spans);
    let bar = Paragraph::new(line).style(Style::default().bg(style.statusbar_bg));
    f.render_widget(bar, area);
}
