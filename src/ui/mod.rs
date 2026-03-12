pub mod dialogs;
pub mod extensions;
pub mod file_tree;
pub mod menu;
pub mod statusbar;
pub mod style;
pub mod treemap;

use ratatui::layout::{Constraint, Direction, Layout};
use ratatui::Frame;

use crate::app::{ActiveTab, App};
use crate::ui::style::style_by_index;

pub fn draw(f: &mut Frame, app: &mut App) {
    let size = f.area();
    let ui_style = style_by_index(app.current_style_index);

    // Main layout: menu bar (1 line), content, status bar (1 line)
    let main_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1), // menu bar
            Constraint::Min(10),   // content
            Constraint::Length(1), // status bar
        ])
        .split(size);

    let menu_bar_area = main_chunks[0];
    menu::draw_menu_bar(f, app, menu_bar_area, &ui_style);
    statusbar::draw(f, app, main_chunks[2], &ui_style);

    match app.active_tab {
        ActiveTab::TreeMap => draw_tree_map_tab(f, app, main_chunks[1], &ui_style),
        ActiveTab::Extensions => extensions::draw(f, app, main_chunks[1], &ui_style),
        ActiveTab::Duplicates => dialogs::draw_duplicates(f, app, main_chunks[1], &ui_style),
    }

    // Draw overlay dialogs
    if app.show_help {
        dialogs::draw_help(f, app, &ui_style);
    }
    if app.confirm_delete.is_some() {
        dialogs::draw_confirm_delete(f, app, &ui_style);
    }
    if app.path_input.is_some() {
        dialogs::draw_path_input(f, app, &ui_style);
    }
    if app.search_input.is_some() {
        dialogs::draw_search_input(f, app);
    }
    if app.top_files_visible {
        dialogs::draw_top_files(f, app, &ui_style);
    }

    // Draw menu dropdown overlay (on top of everything except search bar)
    menu::draw_dropdown(f, app, menu_bar_area, &ui_style);
}

fn draw_tree_map_tab(
    f: &mut Frame,
    app: &mut App,
    area: ratatui::layout::Rect,
    ui_style: &style::UiStyle,
) {
    // Below: extension summary
    let vert_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Min(10),   // top area
            Constraint::Length(8), // extension summary
        ])
        .split(area);

    // Store content area for mouse→pct conversion
    app.content_area = vert_chunks[0];

    if app.show_treemap {
        let horiz_chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Percentage(app.split_pct),
                Constraint::Percentage(100 - app.split_pct),
            ])
            .split(vert_chunks[0]);

        // Store separator x for mouse hit detection
        app.last_split_x = horiz_chunks[0].x + horiz_chunks[0].width;

        let tree_height = horiz_chunks[0].height.saturating_sub(2) as usize;
        app.ensure_visible(tree_height);

        file_tree::draw(f, app, horiz_chunks[0], ui_style);
        let hits = treemap::draw(f, app, horiz_chunks[1], ui_style);
        app.treemap_hits = hits;
    } else {
        // Treemap hidden: tree takes full width
        let tree_height = vert_chunks[0].height.saturating_sub(2) as usize;
        app.ensure_visible(tree_height);

        file_tree::draw(f, app, vert_chunks[0], ui_style);
        app.treemap_hits.clear();
    }

    extensions::draw_summary(f, app, vert_chunks[1], ui_style);
}
