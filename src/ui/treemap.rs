use std::collections::HashMap;

use ratatui::Frame;
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::{Color, Style};
use ratatui::widgets::{Block, Borders};

use indextree::NodeId;

use crate::app::App;
use crate::treemap_algo::{TreemapRect, squarify};
use crate::types::FileTree;
use crate::ui::style::UiStyle;

/// Maximum recursion depth for filling subdirectories in the treemap.
/// Beyond this depth, directories are filled with a single averaged color.
const MAX_DEPTH: u32 = 5;

/// A hit region recorded during treemap drawing for mouse click detection.
#[derive(Clone)]
pub struct TreemapHit {
    pub node_id: NodeId,
    pub x: u16,
    pub y: u16,
    pub w: u16,
    pub h: u16,
}

/// Choose a readable text color (black or white) based on background brightness.
fn label_fg(bg: Color) -> Color {
    match bg {
        Color::Rgb(r, g, b) => {
            let luma = r as u16 * 3 + g as u16 * 4 + b as u16;
            if luma > 900 {
                Color::Black
            } else {
                Color::White
            }
        }
        _ => Color::White,
    }
}

/// Get color for a file node based on its extension.
fn ext_color(ext: &Option<String>, color_map: &HashMap<String, Color>) -> Color {
    match ext {
        Some(e) => color_map.get(e).copied().unwrap_or(Color::Rgb(120, 120, 120)),
        None => Color::Rgb(120, 120, 120),
    }
}

pub fn draw(f: &mut Frame, app: &App, area: Rect, ui_style: &UiStyle) -> Vec<TreemapHit> {
    let mut hits: Vec<TreemapHit> = Vec::new();

    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(ui_style.border_type)
        .title(" Treemap ")
        .border_style(Style::default().fg(ui_style.border_color));

    let inner = block.inner(area);
    f.render_widget(block, area);

    let tree = match &app.tree {
        Some(t) => t,
        None => return hits,
    };

    let root = match app.treemap_root {
        Some(r) => r,
        None => return hits,
    };

    if inner.width == 0 || inner.height == 0 {
        return hits;
    }

    let color_map = &app.ext_color_map;
    let empty_bg = ui_style.treemap_empty_bg;

    // Fill background
    let buf = f.buffer_mut();
    for y in 0..inner.height {
        for x in 0..inner.width {
            buf[(inner.x + x, inner.y + y)]
                .set_char(' ')
                .set_bg(empty_bg);
        }
    }

    let children = tree.sorted_children(root);
    let sizes: Vec<(usize, f64)> = children
        .iter()
        .enumerate()
        .filter_map(|(i, &node_id)| {
            let size = tree.arena[node_id].get().size;
            if size > 0 {
                Some((i, size as f64))
            } else {
                None
            }
        })
        .collect();

    if sizes.is_empty() {
        return hits;
    }

    let layout_rect = TreemapRect {
        x: 0.0,
        y: 0.0,
        w: inner.width as f64,
        h: inner.height as f64,
    };

    let layout = squarify(layout_rect, &sizes);

    let buf = f.buffer_mut();

    // Pass 1: fill colored cells recursively
    for item in &layout {
        let child_node = children[item.index];
        let entry = tree.arena[child_node].get();

        if entry.is_dir {
            fill_subtree(buf, inner, tree, child_node, item.rect, color_map, 1);
        } else {
            let color = ext_color(&entry.extension, color_map);
            fill_rect_bg(buf, inner, item.rect, color);
        }

        // Record hit region
        let hx = (item.rect.x as u16).min(inner.width);
        let hy = (item.rect.y as u16).min(inner.height);
        let hw = (item.rect.w as u16).min(inner.width - hx);
        let hh = (item.rect.h as u16).min(inner.height - hy);
        hits.push(TreemapHit {
            node_id: child_node,
            x: inner.x + hx,
            y: inner.y + hy,
            w: hw,
            h: hh,
        });
    }

    // Pass 2: draw labels on large enough rectangles
    let buf = f.buffer_mut();
    for item in &layout {
        let child_node = children[item.index];
        let entry = tree.arena[child_node].get();
        draw_label(buf, inner, item.rect, &entry.name, entry.size, &entry.extension, entry.is_dir, color_map);
    }

    // Pass 3: selection border
    let buf = f.buffer_mut();
    for item in &layout {
        let child_node = children[item.index];
        if app.treemap_selected == Some(child_node) {
            draw_selection_border(buf, inner, item.rect, ui_style);
        }
    }

    // Collect subtree hits for mouse click on nested files
    for item in &layout {
        let child_node = children[item.index];
        let entry = tree.arena[child_node].get();
        if entry.is_dir {
            collect_subtree_hits(tree, child_node, item.rect, inner, &mut hits, 1);
        }
    }

    hits
}

// ─── Cell rendering ──────────────────────────────────────────

/// Fill a rect with a solid background color (space + bg).
fn fill_rect_bg(buf: &mut Buffer, inner: Rect, r: TreemapRect, color: Color) {
    let x0 = (r.x as u16).min(inner.width);
    let y0 = (r.y as u16).min(inner.height);
    let x1 = ((r.x + r.w) as u16).min(inner.width);
    let y1 = ((r.y + r.h) as u16).min(inner.height);

    for y in y0..y1 {
        for x in x0..x1 {
            let px = inner.x + x;
            let py = inner.y + y;
            if px < inner.x + inner.width && py < inner.y + inner.height {
                buf[(px, py)].set_char(' ').set_bg(color);
            }
        }
    }
}

/// Recursively fill a directory's children into its rect.
fn fill_subtree(
    buf: &mut Buffer,
    inner: Rect,
    tree: &FileTree,
    node_id: NodeId,
    parent_rect: TreemapRect,
    color_map: &HashMap<String, Color>,
    depth: u32,
) {
    // Skip rects too small to see
    if parent_rect.w < 1.0 || parent_rect.h < 1.0 {
        return;
    }

    // At max depth, fill with the dominant extension color
    if depth >= MAX_DEPTH {
        let color = dominant_color(tree, node_id, color_map);
        fill_rect_bg(buf, inner, parent_rect, color);
        return;
    }

    let children = tree.sorted_children(node_id);
    let sizes: Vec<(usize, f64)> = children
        .iter()
        .enumerate()
        .filter_map(|(i, &nid)| {
            let size = tree.arena[nid].get().size;
            if size > 0 {
                Some((i, size as f64))
            } else {
                None
            }
        })
        .collect();

    if sizes.is_empty() {
        return;
    }

    let layout = squarify(parent_rect, &sizes);

    for item in &layout {
        let child_node = children[item.index];
        let entry = tree.arena[child_node].get();

        if entry.is_dir {
            fill_subtree(buf, inner, tree, child_node, item.rect, color_map, depth + 1);
        } else {
            let color = ext_color(&entry.extension, color_map);
            fill_rect_bg(buf, inner, item.rect, color);
        }
    }
}

/// Get the dominant file color in a subtree (largest file's extension).
fn dominant_color(tree: &FileTree, node_id: NodeId, color_map: &HashMap<String, Color>) -> Color {
    // Find the largest file in immediate children
    let mut best_size = 0u64;
    let mut best_color = Color::Rgb(120, 120, 120);
    for child in node_id.children(&tree.arena) {
        let entry = tree.arena[child].get();
        if !entry.is_dir && entry.size > best_size {
            best_size = entry.size;
            best_color = ext_color(&entry.extension, color_map);
        }
    }
    // If no direct files, recurse into largest child dir
    if best_size == 0 {
        for child in node_id.children(&tree.arena) {
            let entry = tree.arena[child].get();
            if entry.is_dir && entry.size > best_size {
                best_size = entry.size;
                best_color = dominant_color(tree, child, color_map);
            }
        }
    }
    best_color
}

/// Draw a label on a rectangle if it's large enough.
fn draw_label(
    buf: &mut Buffer,
    inner: Rect,
    r: TreemapRect,
    name: &str,
    size: u64,
    _extension: &Option<String>,
    _is_dir: bool,
    _color_map: &HashMap<String, Color>,
) {
    let rw = r.w as u16;
    let rh = r.h as u16;

    if rw < 4 || rh < 1 {
        return;
    }

    let rx = r.x as u16;
    let ry = r.y as u16;
    let x = inner.x + rx;
    let y = inner.y + ry;

    if y >= inner.y + inner.height || x >= inner.x + inner.width {
        return;
    }

    // Read the background color from the cell to use as label bg
    let bg_color = buf[(x, y)].bg;
    let fg = label_fg(bg_color);
    let style = Style::default().fg(fg).bg(bg_color);

    // Name label
    let max_len = (rw as usize).saturating_sub(1);
    let label: &str = if name.len() > max_len {
        &name[..max_len]
    } else {
        name
    };

    for (i, ch) in label.chars().enumerate() {
        let px = x + i as u16;
        if px < inner.x + inner.width {
            buf[(px, y)].set_char(ch).set_style(style);
        }
    }

    // Size label on second line
    if rh >= 2 && rw >= 5 {
        let size_str = format_size(size);
        let size_label: &str = if size_str.len() > max_len {
            &size_str[..max_len]
        } else {
            &size_str
        };
        let y2 = y + 1;
        if y2 < inner.y + inner.height {
            for (i, ch) in size_label.chars().enumerate() {
                let px = x + i as u16;
                if px < inner.x + inner.width {
                    buf[(px, y2)].set_char(ch).set_style(style);
                }
            }
        }
    }

}

// ─── Hit regions ──────────────────────────────────────────────

fn collect_subtree_hits(
    tree: &FileTree,
    node_id: NodeId,
    parent_rect: TreemapRect,
    inner: Rect,
    hits: &mut Vec<TreemapHit>,
    depth: u32,
) {
    if depth >= MAX_DEPTH || parent_rect.w < 1.0 || parent_rect.h < 1.0 {
        return;
    }

    let children = tree.sorted_children(node_id);
    let sizes: Vec<(usize, f64)> = children
        .iter()
        .enumerate()
        .filter_map(|(i, &nid)| {
            let size = tree.arena[nid].get().size;
            if size > 0 {
                Some((i, size as f64))
            } else {
                None
            }
        })
        .collect();

    if sizes.is_empty() {
        return;
    }

    let layout = squarify(parent_rect, &sizes);

    for item in &layout {
        let child_node = children[item.index];
        let rx = (item.rect.x as u16).min(inner.width);
        let ry = (item.rect.y as u16).min(inner.height);
        let rw = (item.rect.w as u16).min(inner.width - rx);
        let rh = (item.rect.h as u16).min(inner.height - ry);
        hits.push(TreemapHit {
            node_id: child_node,
            x: inner.x + rx,
            y: inner.y + ry,
            w: rw,
            h: rh,
        });

        let entry = tree.arena[child_node].get();
        if entry.is_dir {
            collect_subtree_hits(tree, child_node, item.rect, inner, hits, depth + 1);
        }
    }
}

// ─── Selection border ─────────────────────────────────────────

fn draw_selection_border(buf: &mut Buffer, area: Rect, rect: TreemapRect, ui_style: &UiStyle) {
    let x_start = (rect.x as u16).min(area.width);
    let y_start = (rect.y as u16).min(area.height);
    let x_end = ((rect.x + rect.w) as u16).min(area.width);
    let y_end = ((rect.y + rect.h) as u16).min(area.height);

    let style = Style::default().fg(Color::White).bg(Color::Rgb(255, 255, 255));

    // Top and bottom edges
    for x in x_start..x_end {
        let px = area.x + x;
        if y_start < area.height {
            let py = area.y + y_start;
            if px < area.x + area.width && py < area.y + area.height {
                buf[(px, py)].set_style(style).set_char(ui_style.sel_h);
            }
        }
        if y_end > 0 {
            let py = area.y + y_end.saturating_sub(1);
            if px < area.x + area.width && py < area.y + area.height {
                buf[(px, py)].set_style(style).set_char(ui_style.sel_h);
            }
        }
    }

    // Left and right edges
    for y in y_start..y_end {
        let py = area.y + y;
        if x_start < area.width {
            let px = area.x + x_start;
            if px < area.x + area.width && py < area.y + area.height {
                buf[(px, py)].set_style(style).set_char(ui_style.sel_v);
            }
        }
        if x_end > 0 {
            let px = area.x + x_end.saturating_sub(1);
            if px < area.x + area.width && py < area.y + area.height {
                buf[(px, py)].set_style(style).set_char(ui_style.sel_v);
            }
        }
    }

    // Corners
    let set_corner = |buf: &mut Buffer, x: u16, y: u16, ch: char| {
        if x < area.x + area.width && y < area.y + area.height {
            buf[(x, y)].set_style(style).set_char(ch);
        }
    };
    set_corner(buf, area.x + x_start, area.y + y_start, ui_style.sel_tl);
    if x_end > 0 {
        set_corner(buf, area.x + x_end.saturating_sub(1), area.y + y_start, ui_style.sel_tr);
    }
    if y_end > 0 {
        set_corner(buf, area.x + x_start, area.y + y_end.saturating_sub(1), ui_style.sel_bl);
    }
    if x_end > 0 && y_end > 0 {
        set_corner(
            buf,
            area.x + x_end.saturating_sub(1),
            area.y + y_end.saturating_sub(1),
            ui_style.sel_br,
        );
    }
}

pub fn format_size(bytes: u64) -> String {
    const KB: u64 = 1024;
    const MB: u64 = 1024 * KB;
    const GB: u64 = 1024 * MB;
    if bytes >= GB {
        format!("{:.1}G", bytes as f64 / GB as f64)
    } else if bytes >= MB {
        format!("{:.1}M", bytes as f64 / MB as f64)
    } else if bytes >= KB {
        format!("{}K", bytes / KB)
    } else {
        format!("{}B", bytes)
    }
}
