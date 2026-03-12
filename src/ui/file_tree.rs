use ratatui::layout::Rect;
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph};
use ratatui::Frame;
use std::borrow::Cow;

use crate::app::App;
use crate::ui::style::UiStyle;
use crate::utils::format_size;

pub fn draw(f: &mut Frame, app: &App, area: Rect, style: &UiStyle) {
    let title = format!(" {} ", app.strings.file_tree);
    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(style.border_type)
        .title(title)
        .border_style(Style::default().fg(style.border_color));

    let inner = block.inner(area);
    f.render_widget(block, area);

    let tree = match &app.tree {
        Some(t) => t,
        None => {
            let msg = Paragraph::new(app.strings.scanning);
            f.render_widget(msg, inner);
            return;
        }
    };

    let root_size = tree.arena[tree.root].get().size.max(1) as f64;
    let visible_height = inner.height as usize;
    let visible_width = inner.width as usize;

    let visible = &app.tree_state.visible_nodes;
    let start = app.tree_state.scroll_offset;
    let end = (start + visible_height).min(visible.len());

    let mut lines: Vec<Line> = Vec::new();

    for (node_id, depth, guide) in &visible[start..end] {
        let node_id = *node_id;
        let depth = *depth;
        let entry = tree.arena[node_id].get();
        let is_selected = app.tree_state.selected == Some(node_id);

        // Build tree connector prefix
        let mut prefix = String::new();
        if depth > 0 {
            for d in 0..(depth as usize - 1) {
                if d + 1 < guide.len() && !guide[d + 1] {
                    prefix.push_str(style.tree_vertical);
                } else {
                    prefix.push_str(&" ".repeat(style.tree_vertical.chars().count()));
                }
            }
            let is_last = guide.last().copied().unwrap_or(false);
            if is_last {
                prefix.push_str(style.tree_last_child);
            } else {
                prefix.push_str(style.tree_branch);
            }
        }

        // Expand/collapse indicator
        let indicator = if entry.is_dir {
            if app.tree_state.expanded.contains(&node_id) {
                style.tree_expanded
            } else {
                style.tree_collapsed
            }
        } else {
            "  "
        };

        // Size and percentage
        let size_str = format_size(entry.size);
        let pct = (entry.size as f64 / root_size * 100.0).min(100.0);
        let pct_str = format!("{:5.1}%", pct);

        // Progress bar (with bounds checking to prevent overflow)
        let bar_width = 10usize;
        let filled = ((pct / 100.0) * bar_width as f64).min(bar_width as f64) as usize;
        let bar: String = format!(
            "{}{}",
            style.bar_filled.repeat(filled),
            style.bar_empty.repeat(bar_width.saturating_sub(filled))
        );

        // Truncate name to fit (UTF-8 safe)
        let meta_len = size_str.len() + pct_str.len() + bar.len() + 4;
        let prefix_len = prefix.chars().count() + indicator.chars().count();
        let name_max = visible_width.saturating_sub(prefix_len + meta_len);
        let name: Cow<str> = if entry.name.chars().count() > name_max {
            let truncated = entry
                .name
                .char_indices()
                .nth(name_max.saturating_sub(1))
                .map(|(i, _)| &entry.name[..i])
                .unwrap_or(&entry.name);
            Cow::Owned(format!("{}~", truncated))
        } else {
            Cow::Borrowed(&entry.name)
        };

        // Pad name to align columns
        let name_padded = format!("{:<width$}", name, width = name_max);

        let node_style = if is_selected {
            Style::default()
                .bg(style.selected_bg)
                .fg(style.selected_fg)
                .add_modifier(Modifier::BOLD)
        } else if entry.is_dir {
            Style::default().fg(style.fg_directory)
        } else {
            Style::default().fg(style.fg_file)
        };

        let pct_color = if pct > 50.0 {
            style.pct_high
        } else if pct > 20.0 {
            style.pct_mid
        } else {
            style.pct_low
        };

        let line = Line::from(vec![
            Span::styled(format!("{}{}", prefix, indicator), node_style),
            Span::styled(name_padded, node_style),
            Span::styled(
                format!(" {} ", size_str),
                Style::default().fg(style.fg_accent),
            ),
            Span::styled(pct_str, Style::default().fg(pct_color)),
            Span::styled(format!(" {}", bar), Style::default().fg(pct_color)),
        ]);

        lines.push(line);
    }

    let paragraph = Paragraph::new(lines);
    f.render_widget(paragraph, inner);
}
