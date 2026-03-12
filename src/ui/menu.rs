use ratatui::layout::Rect;
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, Paragraph};
use ratatui::Frame;

use crate::app::{ActiveTab, App, MenuState};
use crate::ui::style::{all_styles, UiStyle};

/// Menu action returned after a menu item is selected.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MenuAction {
    None,
    OpenDir,
    Rescan,
    ExportCsv,
    Quit,
    SwitchTab(ActiveTab),
    TogglePane,
    ToggleTreemap,
    SetStyle(usize),
    ShowHelp,
}

struct MenuItem {
    label: &'static str,
    shortcut: &'static str,
    action: MenuAction,
}

struct SubMenu {
    label: &'static str,
    items: Vec<MenuItemKind>,
}

enum MenuItemKind {
    Action(MenuItem),
    Separator,
    StylePicker, // special: renders style list dynamically
}

fn build_menus() -> Vec<SubMenu> {
    vec![
        SubMenu {
            label: "File",
            items: vec![
                MenuItemKind::Action(MenuItem {
                    label: "Open dir..",
                    shortcut: "p",
                    action: MenuAction::OpenDir,
                }),
                MenuItemKind::Action(MenuItem {
                    label: "Rescan",
                    shortcut: "r",
                    action: MenuAction::Rescan,
                }),
                MenuItemKind::Action(MenuItem {
                    label: "Export CSV",
                    shortcut: "e",
                    action: MenuAction::ExportCsv,
                }),
                MenuItemKind::Separator,
                MenuItemKind::Action(MenuItem {
                    label: "Quit",
                    shortcut: "q",
                    action: MenuAction::Quit,
                }),
            ],
        },
        SubMenu {
            label: "View",
            items: vec![
                MenuItemKind::Action(MenuItem {
                    label: "Tree+Map",
                    shortcut: "1",
                    action: MenuAction::SwitchTab(ActiveTab::TreeMap),
                }),
                MenuItemKind::Action(MenuItem {
                    label: "Extensions",
                    shortcut: "2",
                    action: MenuAction::SwitchTab(ActiveTab::Extensions),
                }),
                MenuItemKind::Action(MenuItem {
                    label: "Duplicates",
                    shortcut: "3",
                    action: MenuAction::SwitchTab(ActiveTab::Duplicates),
                }),
                MenuItemKind::Separator,
                MenuItemKind::Action(MenuItem {
                    label: "Toggle pane",
                    shortcut: "Tab",
                    action: MenuAction::TogglePane,
                }),
                MenuItemKind::Action(MenuItem {
                    label: "Toggle treemap",
                    shortcut: "t",
                    action: MenuAction::ToggleTreemap,
                }),
            ],
        },
        SubMenu {
            label: "Settings",
            items: vec![MenuItemKind::StylePicker],
        },
        SubMenu {
            label: "Help",
            items: vec![MenuItemKind::Action(MenuItem {
                label: "Shortcuts",
                shortcut: "?",
                action: MenuAction::ShowHelp,
            })],
        },
    ]
}

/// Count selectable items in a submenu.
pub fn item_count(menu_index: usize) -> usize {
    let menus = build_menus();
    if menu_index >= menus.len() {
        return 0;
    }
    let menu = &menus[menu_index];
    let mut count = 0;
    for item in &menu.items {
        match item {
            MenuItemKind::Action(_) => count += 1,
            MenuItemKind::StylePicker => count += all_styles().len(),
            MenuItemKind::Separator => {} // not selectable
        }
    }
    count
}

/// Get the action for a selected item in a submenu.
pub fn item_action(menu_index: usize, item_index: usize, current_style: usize) -> MenuAction {
    let menus = build_menus();
    if menu_index >= menus.len() {
        return MenuAction::None;
    }
    let menu = &menus[menu_index];
    let mut idx = 0;
    for item in &menu.items {
        match item {
            MenuItemKind::Action(mi) => {
                if idx == item_index {
                    return mi.action.clone();
                }
                idx += 1;
            }
            MenuItemKind::StylePicker => {
                let styles = all_styles();
                for (si, _) in styles.iter().enumerate() {
                    if idx == item_index {
                        return MenuAction::SetStyle(si);
                    }
                    idx += 1;
                }
            }
            MenuItemKind::Separator => {}
        }
    }
    let _ = current_style; // used by caller for radio indicators
    MenuAction::None
}

pub const MENU_COUNT: usize = 4;

/// Draw the 1-line menu bar.
pub fn draw_menu_bar(f: &mut Frame, app: &App, area: Rect, style: &UiStyle) {
    let menu_state = &app.menu_state;
    let menus = build_menus();

    let mut spans: Vec<Span> = Vec::new();

    // Left side: menu labels
    for (i, menu) in menus.iter().enumerate() {
        let is_active = menu_state.active && menu_state.selected_menu == i;
        let label = format!(" {} ", menu.label);
        let s = if is_active {
            Style::default()
                .fg(style.menu_active_fg)
                .bg(style.menu_active_bg)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(style.menu_fg).bg(style.menu_bg)
        };
        spans.push(Span::styled(label, s));
    }

    // Right side: tab indicators
    let right_hint = " [1]Tree+Map  [2]Ext  [3]Dupes  [?]Help ";
    let used: usize = spans.iter().map(|s| s.content.len()).sum();
    let remaining = (area.width as usize).saturating_sub(used + right_hint.len());
    spans.push(Span::styled(
        " ".repeat(remaining),
        Style::default().bg(style.menu_bg),
    ));

    // Highlight the active tab indicator
    let tab_s = Style::default()
        .fg(style.menu_shortcut_fg)
        .bg(style.menu_bg);
    let active_tab_s = Style::default()
        .fg(style.menu_fg)
        .bg(style.menu_bg)
        .add_modifier(Modifier::BOLD);

    let tabs = [
        (" [1]Tree+Map ", ActiveTab::TreeMap),
        (" [2]Ext ", ActiveTab::Extensions),
        (" [3]Dupes ", ActiveTab::Duplicates),
    ];
    for (label, tab) in &tabs {
        let s = if app.active_tab == *tab {
            active_tab_s
        } else {
            tab_s
        };
        spans.push(Span::styled(*label, s));
    }
    spans.push(Span::styled(
        " [?]Help ",
        Style::default()
            .fg(style.menu_shortcut_fg)
            .bg(style.menu_bg),
    ));

    let line = Line::from(spans);
    let bar = Paragraph::new(line).style(Style::default().bg(style.menu_bg));
    f.render_widget(bar, area);
}

/// Draw dropdown overlay if menu is open.
pub fn draw_dropdown(f: &mut Frame, app: &App, menu_bar_area: Rect, style: &UiStyle) {
    let menu_state = &app.menu_state;
    if !menu_state.active || !menu_state.dropdown_open {
        return;
    }

    let menus = build_menus();
    if menu_state.selected_menu >= menus.len() {
        return;
    }
    let menu = &menus[menu_state.selected_menu];

    // Calculate dropdown position: below the menu label
    let mut x_offset: u16 = 0;
    for (i, m) in menus.iter().enumerate() {
        if i == menu_state.selected_menu {
            break;
        }
        x_offset += m.label.len() as u16 + 2; // " label "
    }
    let x = menu_bar_area.x + x_offset;

    // Build lines for the dropdown
    let mut lines: Vec<Line> = Vec::new();
    let mut max_width: u16 = 0;
    let mut selectable_idx: usize = 0;
    let style_names = all_styles();

    for item in &menu.items {
        match item {
            MenuItemKind::Action(mi) => {
                let is_sel = menu_state.selected_item == selectable_idx;
                let line = render_menu_item(
                    mi.label,
                    mi.shortcut,
                    is_sel,
                    style,
                    None, // no radio
                );
                let w = mi.label.len() + mi.shortcut.len() + 6; // padding
                max_width = max_width.max(w as u16);
                lines.push(line);
                selectable_idx += 1;
            }
            MenuItemKind::StylePicker => {
                for (si, sname) in style_names.iter().enumerate() {
                    let is_sel = menu_state.selected_item == selectable_idx;
                    let radio = if si == app.current_style_index {
                        Some("●")
                    } else {
                        Some("○")
                    };
                    let line = render_menu_item(sname, "", is_sel, style, radio);
                    let w = sname.len() + 6;
                    max_width = max_width.max(w as u16);
                    lines.push(line);
                    selectable_idx += 1;
                }
            }
            MenuItemKind::Separator => {
                let sep_line = Line::from(Span::styled(
                    "──────────────────────",
                    Style::default()
                        .fg(style.menu_drop_fg)
                        .bg(style.menu_drop_bg),
                ));
                lines.push(sep_line);
            }
        }
    }

    max_width = max_width.max(22);
    let dropdown_width = max_width + 2; // borders
    let dropdown_height = lines.len() as u16 + 2; // borders

    let y = menu_bar_area.y + menu_bar_area.height;

    // Clamp to screen
    let screen = f.area();
    let x = x.min(screen.width.saturating_sub(dropdown_width));
    let dropdown_area = Rect::new(x, y, dropdown_width, dropdown_height.min(screen.height - y));

    f.render_widget(Clear, dropdown_area);

    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(style.border_type)
        .border_style(Style::default().fg(style.border_color));

    let inner = block.inner(dropdown_area);
    f.render_widget(
        block.style(Style::default().bg(style.menu_drop_bg)),
        dropdown_area,
    );

    // Render each line, padded to dropdown width
    let buf = f.buffer_mut();
    for (i, line) in lines.iter().enumerate() {
        let ly = inner.y + i as u16;
        if ly >= inner.y + inner.height {
            break;
        }
        // Clear line bg
        for lx in inner.x..inner.x + inner.width {
            if lx < screen.width && ly < screen.height {
                buf[(lx, ly)]
                    .set_char(' ')
                    .set_style(Style::default().bg(style.menu_drop_bg));
            }
        }
        // Render spans
        let mut cx = inner.x;
        for span in &line.spans {
            for ch in span.content.chars() {
                if cx < inner.x + inner.width {
                    buf[(cx, ly)].set_char(ch).set_style(span.style);
                    cx += 1;
                }
            }
        }
    }
}

fn render_menu_item<'a>(
    label: &str,
    shortcut: &str,
    selected: bool,
    style: &UiStyle,
    radio: Option<&str>,
) -> Line<'a> {
    let (fg, bg) = if selected {
        (style.menu_drop_sel_fg, style.menu_drop_sel_bg)
    } else {
        (style.menu_drop_fg, style.menu_drop_bg)
    };

    let mut spans: Vec<Span> = Vec::new();

    if let Some(r) = radio {
        spans.push(Span::styled(
            format!(" {} ", r),
            Style::default().fg(fg).bg(bg),
        ));
    } else {
        spans.push(Span::styled("  ", Style::default().fg(fg).bg(bg)));
    }

    spans.push(Span::styled(
        label.to_string(),
        Style::default().fg(fg).bg(bg),
    ));

    if !shortcut.is_empty() {
        spans.push(Span::styled(
            format!("  {}", shortcut),
            Style::default()
                .fg(if selected { fg } else { style.menu_shortcut_fg })
                .bg(bg),
        ));
    }

    Line::from(spans)
}

/// Handle menu bar mouse click: returns true if click was on the menu bar.
pub fn handle_menu_click(menu_state: &mut MenuState, x: u16, y: u16, menu_bar_y: u16) -> bool {
    if y != menu_bar_y {
        return false;
    }

    let menus = build_menus();
    let mut offset: u16 = 0;
    for (i, menu) in menus.iter().enumerate() {
        let width = menu.label.len() as u16 + 2;
        if x >= offset && x < offset + width {
            if menu_state.active && menu_state.selected_menu == i && menu_state.dropdown_open {
                // Clicking same menu closes it
                menu_state.active = false;
                menu_state.dropdown_open = false;
            } else {
                menu_state.active = true;
                menu_state.selected_menu = i;
                menu_state.dropdown_open = true;
                menu_state.selected_item = 0;
            }
            return true;
        }
        offset += width;
    }

    // Click outside menus: close
    if menu_state.active {
        menu_state.active = false;
        menu_state.dropdown_open = false;
        return true;
    }

    false
}
