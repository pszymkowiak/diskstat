#![allow(dead_code)]
mod actions;
mod app;
mod config;
mod i18n;
mod json_export;
mod scanner;
mod treemap_algo;
mod types;
mod ui;
mod utils;

use std::env;
use std::io;
use std::path::PathBuf;
use std::sync::mpsc;
use std::time::{Duration, Instant};

use clap::Parser;
use crossterm::event::{
    self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyModifiers,
};
use crossterm::execute;
use crossterm::terminal::{
    disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen,
};
use ratatui::backend::CrosstermBackend;
use ratatui::Terminal;

use app::{ActivePane, ActiveTab, App, ScanState};
use indextree::NodeId;
use scanner::debug_log::DebugLog;
use types::ScanProgress;
use ui::menu::{self, MenuAction};

/// Fast TUI disk usage analyzer — WinDirStat/ncdu alternative
#[derive(Parser)]
#[command(name = "diskstat", version, about)]
struct Cli {
    /// Directory to analyze (default: current directory or last scanned)
    #[arg(value_name = "PATH")]
    path: Option<PathBuf>,

    /// Force a fresh scan (ignore cache)
    #[arg(short = 'f', long = "fresh")]
    fresh: bool,

    /// Show version and exit
    #[arg(long = "info")]
    info: bool,

    /// Exclude directories matching these patterns (can be repeated)
    /// Common patterns: node_modules, .git, target, __pycache__, .cache
    #[arg(short = 'e', long = "exclude", value_name = "PATTERN")]
    exclude: Vec<String>,

    /// Export scan results to JSON (outputs to stdout, no TUI)
    #[arg(long = "json")]
    json: bool,
}

fn main() -> io::Result<()> {
    let cli = Cli::parse();

    if cli.info {
        println!("diskstat {}", env!("CARGO_PKG_VERSION"));
        println!("Fast TUI disk usage analyzer");
        println!("https://github.com/pszymkowiak/diskstat");
        return Ok(());
    }

    // Open global debug log
    let dlog = DebugLog::open().ok();

    // Resolve path: CLI arg > last scanned > current dir
    let root_path = cli
        .path
        .or_else(|| dlog.as_ref().and_then(|d| d.get_last_scanned()))
        .unwrap_or_else(|| env::current_dir().unwrap_or_else(|_| PathBuf::from(".")));

    let root_path = root_path.canonicalize().unwrap_or(root_path);

    if let Some(ref d) = dlog {
        d.log_json(
            "app_start",
            &[
                ("root_path", &root_path.to_string_lossy()),
                ("args", &env::args().collect::<Vec<_>>().join(" ")),
            ],
        );
    }

    // JSON export mode (no TUI)
    if cli.json {
        return run_json_export(root_path, cli.exclude);
    }

    // Setup terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let result = run_app(&mut terminal, root_path, dlog, cli.fresh, cli.exclude);

    // Restore terminal
    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;

    if let Err(e) = result {
        eprintln!("Error: {}", e);
    }

    Ok(())
}

fn run_app(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    root_path: PathBuf,
    dlog: Option<DebugLog>,
    fresh: bool,
    exclude: Vec<String>,
) -> io::Result<()> {
    let mut app = App::new(root_path.clone(), exclude.clone());

    // Try loading from binary cache first (instant startup), unless --fresh
    let start = Instant::now();
    let cached_tree = if !fresh {
        scanner::tree_cache::load_tree(&root_path)
    } else {
        None
    };
    let mut scan_handle = if let Some(tree) = cached_tree {
        let elapsed = start.elapsed();
        let node_count = tree.arena.count();
        if let Some(ref d) = dlog {
            d.log_json(
                "cache_load_ok",
                &[
                    ("root_path", &root_path.to_string_lossy()),
                    ("nodes", &node_count.to_string()),
                    (
                        "elapsed_ms",
                        &format!("{:.1}", elapsed.as_secs_f64() * 1000.0),
                    ),
                ],
            );
            d.set_last_scanned(&root_path);
        }
        app.tree = Some(tree);
        app.scan_state = ScanState::Done;
        app.on_scan_complete();
        app.status_message = Some(app.strings.loaded_from_cache.to_string());
        None
    } else {
        let elapsed = start.elapsed();
        if let Some(ref d) = dlog {
            d.log_json(
                "cache_load_miss",
                &[
                    ("root_path", &root_path.to_string_lossy()),
                    (
                        "elapsed_ms",
                        &format!("{:.1}", elapsed.as_secs_f64() * 1000.0),
                    ),
                ],
            );
        }
        let (progress_tx, progress_rx) = mpsc::channel();
        app.progress_rx = Some(progress_rx);
        app.scan_state = ScanState::Scanning;

        if let Some(ref d) = dlog {
            d.log_json("scan_start", &[("root_path", &root_path.to_string_lossy())]);
        }
        Some(scanner::walk::scan_directory(
            root_path.clone(),
            progress_tx,
            exclude.clone(),
        ))
    };

    loop {
        // Poll progress from scanner
        app.poll_progress();

        // Check if scan thread completed and grab the tree
        if app.scan_state == ScanState::Done && app.tree.is_none() {
            if let Some(handle) = scan_handle.take() {
                match handle.join() {
                    Ok(Some(tree)) => {
                        let node_count = tree.arena.count();
                        let root_size = tree.arena[tree.root].get().size;

                        // Save binary cache for instant reload next time
                        let _ = scanner::tree_cache::save_tree(&tree);

                        if let Some(ref d) = dlog {
                            d.log_json(
                                "scan_complete",
                                &[
                                    ("root_path", &tree.root_path.to_string_lossy()),
                                    ("nodes", &node_count.to_string()),
                                    ("total_size", &root_size.to_string()),
                                ],
                            );
                            d.set_last_scanned(&tree.root_path);
                        }

                        app.tree = Some(tree);
                        app.on_scan_complete();
                        app.needs_redraw = true;
                    }
                    Ok(None) => {
                        if let Some(ref d) = dlog {
                            d.log("scan_failed", "scan_directory returned None");
                        }
                        app.status_message = Some(app.strings.scan_failed.to_string());
                    }
                    Err(_) => {
                        if let Some(ref d) = dlog {
                            d.log("scan_panic", "scan thread panicked");
                        }
                        app.status_message = Some(app.strings.scan_thread_panicked.to_string());
                    }
                }
            }
        }

        // Draw only when needed
        if app.needs_redraw {
            terminal.draw(|f| ui::draw(f, &mut app))?;
            app.needs_redraw = false;
        }

        let poll_timeout = if app.scan_state == ScanState::Scanning {
            Duration::from_millis(50)
        } else {
            Duration::from_millis(200)
        };

        // Poll events
        if event::poll(poll_timeout)? {
            match event::read()? {
                Event::Key(key) => {
                    app.needs_redraw = true;
                    match handle_input(&mut app, key.code, key.modifiers) {
                        InputAction::Quit => {
                            if let Some(ref d) = dlog {
                                d.log("app_quit", "{}");
                                d.trim(5000);
                            }
                            return Ok(());
                        }
                        InputAction::Rescan(new_path) => {
                            if let Some(ref d) = dlog {
                                d.log_json("rescan", &[("root_path", &new_path.to_string_lossy())]);
                            }
                            app.reset_for_scan(new_path.clone());
                            let (tx, rx) = mpsc::channel();
                            app.progress_rx = Some(rx);
                            app.scan_state = ScanState::Scanning;
                            scan_handle = Some(scanner::walk::scan_directory(
                                new_path,
                                tx,
                                app.exclude_patterns.clone(),
                            ));
                        }
                        InputAction::ForceRescan(path) => {
                            if let Some(ref d) = dlog {
                                d.log_json(
                                    "force_rescan",
                                    &[("root_path", &path.to_string_lossy())],
                                );
                            }
                            // Invalidate both caches then rescan
                            scanner::tree_cache::invalidate(&path);
                            if let Ok(mut cache) = scanner::cache::ScanCache::open(&path) {
                                let _ = cache.invalidate_all();
                            }
                            app.reset_for_scan(path.clone());
                            let (tx, rx) = mpsc::channel();
                            app.progress_rx = Some(rx);
                            app.scan_state = ScanState::Scanning;
                            scan_handle = Some(scanner::walk::scan_directory(
                                path,
                                tx,
                                app.exclude_patterns.clone(),
                            ));
                        }
                        InputAction::SubtreeRescan(target_node, subtree_path) => {
                            if let Some(ref d) = dlog {
                                d.log_json(
                                    "subtree_rescan",
                                    &[("path", &subtree_path.to_string_lossy())],
                                );
                            }
                            app.status_message =
                                Some(format!("Rescanning {}...", subtree_path.display()));
                            // Perform subtree rescan synchronously (blocking)
                            let (tx, _rx) = mpsc::channel();
                            let handle = scanner::walk::scan_directory(
                                subtree_path,
                                tx,
                                app.exclude_patterns.clone(),
                            );
                            if let Ok(Some(mini_tree)) = handle.join() {
                                if let Some(tree) = &mut app.tree {
                                    // Remove old children of target node
                                    let old_children: Vec<_> =
                                        target_node.children(&tree.arena).collect();
                                    for child in old_children {
                                        child.detach(&mut tree.arena);
                                    }
                                    // Graft new children from mini_tree
                                    graft_children(tree, target_node, &mini_tree, mini_tree.root);
                                    tree.compute_sizes();
                                }
                                // Recompute extension stats
                                if let Some(tree) = &app.tree {
                                    let stats = scanner::tree::compute_extension_stats(tree);
                                    let color_map = scanner::tree::extension_color_map(&stats);
                                    app.ext_stats = stats;
                                    app.ext_color_map = color_map;

                                    let root_entry = tree.arena[tree.root].get();
                                    app.file_count = root_entry.file_count;
                                    app.total_size = root_entry.size;
                                }
                                app.rebuild_visible_nodes();
                                app.subtree_target = None;
                                app.status_message = Some("Subtree rescan complete".to_string());
                            } else {
                                app.subtree_target = None;
                                app.status_message = Some("Subtree rescan failed".to_string());
                            }
                        }
                        InputAction::Continue => {}
                    }
                }
                Event::Mouse(mouse) => {
                    app.needs_redraw = true;
                    handle_mouse(&mut app, mouse);
                }
                Event::Resize(_, _) => {
                    app.needs_redraw = true;
                }
                _ => {}
            }
        }
    }
}

/// Return value: (should_quit, new_scan_path)
fn handle_input(app: &mut App, code: KeyCode, modifiers: KeyModifiers) -> InputAction {
    // Handle dialogs first (highest priority)
    if app.show_help {
        app.show_help = false;
        return InputAction::Continue;
    }

    // Top files dialog
    if app.top_files_visible {
        return handle_top_files_input(app, code, modifiers);
    }

    // Search input
    if app.search_input.is_some() {
        return handle_search_input(app, code, modifiers);
    }

    // Filter input dialog
    if app.filter_input.is_some() {
        return handle_filter_input(app, code, modifiers);
    }

    // Path input dialog
    if app.path_input.is_some() {
        return handle_path_input(app, code, modifiers);
    }

    if app.confirm_delete.is_some() {
        match code {
            KeyCode::Char('y') | KeyCode::Char('Y') => {
                if let Some((path, _size)) = app.confirm_delete.take() {
                    match actions::delete_path(&path, &app.root_path) {
                        Ok(()) => {
                            app.status_message = Some(format!("Deleted: {}", path.display()));
                            return InputAction::ForceRescan(app.root_path.clone());
                        }
                        Err(e) => {
                            app.status_message = Some(e);
                        }
                    }
                }
            }
            _ => {
                app.confirm_delete = None;
            }
        }
        return InputAction::Continue;
    }

    // Menu input (after dialogs, before global keys)
    if app.menu_state.active {
        return handle_menu_input(app, code, modifiers);
    }

    // F10 toggles menu
    if code == KeyCode::F(10) {
        app.menu_state.active = true;
        app.menu_state.selected_menu = 0;
        app.menu_state.dropdown_open = false;
        app.menu_state.selected_item = 0;
        return InputAction::Continue;
    }

    // Ctrl+C always quits
    if modifiers.contains(KeyModifiers::CONTROL) && code == KeyCode::Char('c') {
        return InputAction::Quit;
    }

    match code {
        KeyCode::Char('q') | KeyCode::Esc => return InputAction::Quit,
        KeyCode::Char('?') => app.show_help = !app.show_help,

        // Tab switching
        KeyCode::Char('1') => app.active_tab = ActiveTab::TreeMap,
        KeyCode::Char('2') => app.active_tab = ActiveTab::Extensions,
        KeyCode::Char('3') => app.active_tab = ActiveTab::Duplicates,
        KeyCode::Tab => {
            app.active_pane = match app.active_pane {
                ActivePane::Tree => ActivePane::Map,
                ActivePane::Map => ActivePane::Tree,
            };
        }

        // Tree navigation
        KeyCode::Up | KeyCode::Char('k') => match app.active_tab {
            ActiveTab::TreeMap => app.tree_up(),
            ActiveTab::Extensions => {
                if app.ext_selected_index > 0 {
                    app.ext_selected_index -= 1;
                }
            }
            ActiveTab::Duplicates => {
                if app.dupes_selected_index > 0 {
                    app.dupes_selected_index -= 1;
                }
            }
        },
        KeyCode::Down | KeyCode::Char('j') => match app.active_tab {
            ActiveTab::TreeMap => app.tree_down(),
            ActiveTab::Extensions => {
                if app.ext_selected_index + 1 < app.ext_stats.len() {
                    app.ext_selected_index += 1;
                }
            }
            ActiveTab::Duplicates => {
                if app.dupes_selected_index + 1 < app.duplicates.len() {
                    app.dupes_selected_index += 1;
                }
            }
        },
        KeyCode::Left | KeyCode::Char('h') => app.tree_collapse(),
        KeyCode::Right | KeyCode::Char('l') => app.tree_expand(),

        // Treemap zoom
        KeyCode::Enter => app.treemap_enter(),
        KeyCode::Backspace => app.treemap_back(),

        // Actions
        KeyCode::Char('d') | KeyCode::Delete => {
            if let Some(selected) = app.tree_state.selected {
                if let Some(tree) = &app.tree {
                    let size = tree.arena[selected].get().size;
                    if let Some(path) = app.selected_path() {
                        app.confirm_delete = Some((path, size));
                    }
                }
            }
        }
        KeyCode::Char('o') => {
            if let Some(path) = app.selected_path() {
                match actions::open_in_finder(&path) {
                    Ok(()) => app.status_message = Some(format!("Opened: {}", path.display())),
                    Err(e) => app.status_message = Some(e),
                }
            }
        }
        KeyCode::Char('c') => {
            if let Some(path) = app.selected_path() {
                match actions::copy_to_clipboard(&path) {
                    Ok(()) => app.status_message = Some("Path copied to clipboard".to_string()),
                    Err(e) => app.status_message = Some(e),
                }
            }
        }
        KeyCode::Char('s') => {
            if app.active_tab == ActiveTab::Duplicates && !app.dupes_scanning {
                // Scan for duplicates
                if let Some(tree) = &app.tree {
                    app.dupes_scanning = true;
                    app.status_message = Some("Scanning for duplicates...".to_string());
                    let dupes = scanner::dupes::find_duplicates(tree);
                    app.duplicates = dupes;
                    app.dupes_scanning = false;
                    app.status_message =
                        Some(format!("Found {} duplicate groups", app.duplicates.len()));
                }
            } else if app.active_tab == ActiveTab::TreeMap {
                // Cycle sort mode
                app.sort_mode = app.sort_mode.next();
                app.rebuild_visible_nodes();
                app.status_message = Some(format!("Sort: {}", app.sort_mode.display_name()));
            }
        }

        // Change directory
        KeyCode::Char('p') => {
            app.open_path_input();
        }

        // Rescan current directory (force, ignores cache)
        KeyCode::Char('r') => {
            return InputAction::ForceRescan(app.root_path.clone());
        }

        // Search (/ like vim)
        KeyCode::Char('/') => {
            app.search_input = Some(String::new());
        }

        // Next/prev search match
        KeyCode::Char('n') => {
            if app.search_query.is_some() {
                app.search_next();
            }
        }
        KeyCode::Char('N') => {
            if app.search_query.is_some() {
                app.search_prev();
            }
        }

        // Export CSV
        KeyCode::Char('e') => {
            if let Some(tree) = &app.tree {
                match actions::export_csv(tree) {
                    Ok(path) => {
                        app.status_message = Some(format!("Exported to {}", path));
                    }
                    Err(e) => {
                        app.status_message = Some(e);
                    }
                }
            }
        }

        // Toggle treemap panel
        KeyCode::Char('t') => {
            app.show_treemap = !app.show_treemap;
        }

        // Top files view
        KeyCode::Char('f') => {
            app.top_files_visible = !app.top_files_visible;
            if app.top_files_visible {
                app.compute_top_files();
            }
        }

        // Rescan subtree
        KeyCode::Char('R') => {
            if let Some(selected) = app.tree_state.selected {
                if let Some(tree) = &app.tree {
                    if tree.arena[selected].get().is_dir && selected != tree.root {
                        app.subtree_target = Some(selected);
                        let subtree_path = tree.full_path(selected);
                        return InputAction::SubtreeRescan(selected, subtree_path);
                    }
                }
            }
        }

        // Filter by size
        KeyCode::Char('F') => {
            app.filter_input = Some(String::new());
        }

        // Clear filter
        KeyCode::Char('C') => {
            if app.min_size_filter.is_some() {
                app.min_size_filter = None;
                app.rebuild_visible_nodes();
                app.status_message = Some(app.strings.filter_cleared.to_string());
            }
        }

        _ => {}
    }

    InputAction::Continue
}

fn handle_top_files_input(app: &mut App, code: KeyCode, modifiers: KeyModifiers) -> InputAction {
    // Ctrl+C or Esc to close
    if modifiers.contains(KeyModifiers::CONTROL) && code == KeyCode::Char('c') {
        return InputAction::Quit;
    }

    match code {
        KeyCode::Esc | KeyCode::Char('f') => {
            app.top_files_visible = false;
        }
        KeyCode::Up | KeyCode::Char('k') => {
            if app.top_files_selected > 0 {
                app.top_files_selected -= 1;
            }
        }
        KeyCode::Down | KeyCode::Char('j') => {
            if app.top_files_selected + 1 < app.top_files.len() {
                app.top_files_selected += 1;
            }
        }
        KeyCode::Enter => {
            // Navigate to selected file in tree
            if let Some(&(node_id, _)) = app.top_files.get(app.top_files_selected) {
                app.expand_to_node(node_id);
                app.top_files_visible = false;
            }
        }
        _ => {}
    }

    InputAction::Continue
}

fn handle_menu_input(app: &mut App, code: KeyCode, modifiers: KeyModifiers) -> InputAction {
    // Ctrl+C always quits even in menu
    if modifiers.contains(KeyModifiers::CONTROL) && code == KeyCode::Char('c') {
        return InputAction::Quit;
    }

    match code {
        KeyCode::Esc | KeyCode::F(10) => {
            app.menu_state.active = false;
            app.menu_state.dropdown_open = false;
        }
        KeyCode::Left => {
            if app.menu_state.selected_menu > 0 {
                app.menu_state.selected_menu -= 1;
            } else {
                app.menu_state.selected_menu = menu::MENU_COUNT - 1;
            }
            app.menu_state.selected_item = 0;
        }
        KeyCode::Right => {
            app.menu_state.selected_menu = (app.menu_state.selected_menu + 1) % menu::MENU_COUNT;
            app.menu_state.selected_item = 0;
        }
        KeyCode::Down => {
            if !app.menu_state.dropdown_open {
                app.menu_state.dropdown_open = true;
                app.menu_state.selected_item = 0;
            } else {
                let count = menu::item_count(app.menu_state.selected_menu);
                if count > 0 {
                    app.menu_state.selected_item = (app.menu_state.selected_item + 1) % count;
                }
            }
        }
        KeyCode::Up => {
            if app.menu_state.dropdown_open {
                let count = menu::item_count(app.menu_state.selected_menu);
                if count > 0 {
                    if app.menu_state.selected_item == 0 {
                        app.menu_state.selected_item = count - 1;
                    } else {
                        app.menu_state.selected_item -= 1;
                    }
                }
            }
        }
        KeyCode::Enter => {
            if !app.menu_state.dropdown_open {
                app.menu_state.dropdown_open = true;
                app.menu_state.selected_item = 0;
            } else {
                let action = menu::item_action(
                    app.menu_state.selected_menu,
                    app.menu_state.selected_item,
                    app.current_style_index,
                );
                app.menu_state.active = false;
                app.menu_state.dropdown_open = false;
                return dispatch_menu_action(app, action);
            }
        }
        _ => {}
    }

    InputAction::Continue
}

fn dispatch_menu_action(app: &mut App, action: MenuAction) -> InputAction {
    match action {
        MenuAction::None => InputAction::Continue,
        MenuAction::OpenDir => {
            app.open_path_input();
            InputAction::Continue
        }
        MenuAction::Rescan => InputAction::ForceRescan(app.root_path.clone()),
        MenuAction::ExportCsv => {
            if let Some(tree) = &app.tree {
                match actions::export_csv(tree) {
                    Ok(path) => app.status_message = Some(format!("Exported to {}", path)),
                    Err(e) => app.status_message = Some(e),
                }
            }
            InputAction::Continue
        }
        MenuAction::Quit => InputAction::Quit,
        MenuAction::SwitchTab(tab) => {
            app.active_tab = tab;
            InputAction::Continue
        }
        MenuAction::TogglePane => {
            app.active_pane = match app.active_pane {
                ActivePane::Tree => ActivePane::Map,
                ActivePane::Map => ActivePane::Tree,
            };
            InputAction::Continue
        }
        MenuAction::ToggleTreemap => {
            app.show_treemap = !app.show_treemap;
            InputAction::Continue
        }
        MenuAction::SetStyle(idx) => {
            app.current_style_index = idx;
            let names = ui::style::all_styles();
            app.status_message = Some(format!("Style: {}", names[idx]));
            InputAction::Continue
        }
        MenuAction::ShowHelp => {
            app.show_help = true;
            InputAction::Continue
        }
    }
}

fn handle_path_input(app: &mut App, code: KeyCode, modifiers: KeyModifiers) -> InputAction {
    // Ctrl+C cancels
    if modifiers.contains(KeyModifiers::CONTROL) && code == KeyCode::Char('c') {
        app.path_input = None;
        return InputAction::Continue;
    }

    let input = app.path_input.as_mut().unwrap();

    match code {
        KeyCode::Esc => {
            app.path_input = None;
        }
        KeyCode::Enter => {
            if let Some(path) = input.validate() {
                app.path_input = None;
                return InputAction::Rescan(path);
            } else {
                app.status_message = Some("Invalid directory path".to_string());
            }
        }
        KeyCode::Tab => {
            input.complete();
        }
        KeyCode::Backspace => {
            input.backspace();
        }
        KeyCode::Delete => {
            input.delete();
        }
        KeyCode::Left => {
            input.move_left();
        }
        KeyCode::Right => {
            input.move_right();
        }
        KeyCode::Home => {
            input.move_home();
        }
        KeyCode::End => {
            input.move_end();
        }
        KeyCode::Char(c) => {
            input.insert_char(c);
        }
        _ => {}
    }

    InputAction::Continue
}

fn handle_search_input(app: &mut App, code: KeyCode, modifiers: KeyModifiers) -> InputAction {
    // Ctrl+C cancels
    if modifiers.contains(KeyModifiers::CONTROL) && code == KeyCode::Char('c') {
        app.search_input = None;
        return InputAction::Continue;
    }

    match code {
        KeyCode::Esc => {
            app.search_input = None;
            app.search_query = None;
            app.search_matches.clear();
            app.search_index = 0;
        }
        KeyCode::Enter => {
            if let Some(input) = app.search_input.take() {
                if !input.is_empty() {
                    app.search_query = Some(input);
                    app.search_execute();
                }
            }
        }
        KeyCode::Backspace => {
            if let Some(input) = app.search_input.as_mut() {
                input.pop();
            }
        }
        KeyCode::Char(c) => {
            if let Some(input) = app.search_input.as_mut() {
                input.push(c);
            }
        }
        _ => {}
    }

    InputAction::Continue
}

fn handle_filter_input(app: &mut App, code: KeyCode, modifiers: KeyModifiers) -> InputAction {
    // Ctrl+C cancels
    if modifiers.contains(KeyModifiers::CONTROL) && code == KeyCode::Char('c') {
        app.filter_input = None;
        return InputAction::Continue;
    }

    match code {
        KeyCode::Esc => {
            app.filter_input = None;
        }
        KeyCode::Enter => {
            if let Some(input) = app.filter_input.take() {
                if !input.is_empty() {
                    // Parse size with units (e.g., "10M", "1.5G", "500K")
                    if let Some(bytes) = parse_size(&input) {
                        app.min_size_filter = Some(bytes);
                        app.rebuild_visible_nodes();
                        let formatted = bytesize::ByteSize(bytes).to_string();
                        app.status_message = Some(format!(
                            "{}: {}",
                            app.strings.filter_active.replace("{}", &formatted),
                            formatted
                        ));
                    } else {
                        app.status_message =
                            Some("Invalid size format (use 10M, 1.5G, 500K)".to_string());
                    }
                } else {
                    // Empty input = clear filter
                    app.min_size_filter = None;
                    app.rebuild_visible_nodes();
                    app.status_message = Some(app.strings.filter_cleared.to_string());
                }
            }
        }
        KeyCode::Backspace => {
            if let Some(input) = app.filter_input.as_mut() {
                input.pop();
            }
        }
        KeyCode::Char(c) => {
            if let Some(input) = app.filter_input.as_mut() {
                input.push(c);
            }
        }
        _ => {}
    }

    InputAction::Continue
}

/// Parse size string like "10M", "1.5G", "500K" into bytes
fn parse_size(s: &str) -> Option<u64> {
    let s = s.trim().to_uppercase();
    if s.is_empty() {
        return None;
    }

    // Find the boundary between number and unit
    let mut num_end = 0;
    for (i, c) in s.chars().enumerate() {
        if c.is_ascii_digit() || c == '.' {
            num_end = i + 1;
        } else {
            break;
        }
    }

    let (num_str, unit) = s.split_at(num_end);
    let num: f64 = num_str.parse().ok()?;

    let multiplier: u64 = match unit.trim() {
        "" | "B" => 1,
        "K" | "KB" => 1024,
        "M" | "MB" => 1024 * 1024,
        "G" | "GB" => 1024 * 1024 * 1024,
        "T" | "TB" => 1024_u64 * 1024 * 1024 * 1024,
        _ => return None,
    };

    Some((num * multiplier as f64) as u64)
}

enum InputAction {
    Continue,
    Quit,
    Rescan(PathBuf),
    ForceRescan(PathBuf),
    SubtreeRescan(indextree::NodeId, PathBuf),
}

fn handle_mouse(app: &mut App, mouse: crossterm::event::MouseEvent) {
    use crossterm::event::{MouseButton, MouseEventKind};

    let mx = mouse.column;
    let my = mouse.row;

    match mouse.kind {
        MouseEventKind::Down(MouseButton::Left) => {
            // Check menu bar click first (row 0 = menu bar)
            if menu::handle_menu_click(&mut app.menu_state, mx, my, 0) {
                return;
            }

            // If menu is open and clicked elsewhere, close it
            if app.menu_state.active {
                app.menu_state.active = false;
                app.menu_state.dropdown_open = false;
                return;
            }

            // Check if clicking on the split separator (±1 col tolerance)
            if app.show_treemap && app.active_tab == ActiveTab::TreeMap {
                let sep = app.last_split_x;
                let area = app.content_area;
                if mx >= sep.saturating_sub(1)
                    && mx <= sep.saturating_add(1)
                    && my >= area.y
                    && my < area.y + area.height
                {
                    app.dragging_split = true;
                    return;
                }
            }

            app.status_message = None;

            // Hit-test treemap: iterate in reverse (last = smallest/deepest rectangle)
            for hit in app.treemap_hits.iter().rev() {
                if mx >= hit.x && mx < hit.x + hit.w && my >= hit.y && my < hit.y + hit.h {
                    app.expand_to_node(hit.node_id);
                    break;
                }
            }
        }
        MouseEventKind::Drag(MouseButton::Left) => {
            if app.dragging_split {
                let area = app.content_area;
                if area.width > 0 {
                    let relative = mx.saturating_sub(area.x);
                    let pct = (relative as u32 * 100 / area.width as u32) as u16;
                    app.split_pct = pct.clamp(10, 90);
                }
            }
        }
        MouseEventKind::Up(MouseButton::Left) => {
            app.dragging_split = false;
        }
        _ => {}
    }
}

/// Graft children from a mini-tree (from subtree rescan) into the main tree.
fn graft_children(
    tree: &mut crate::types::FileTree,
    target: indextree::NodeId,
    src: &crate::types::FileTree,
    src_root: indextree::NodeId,
) {
    for src_child in src_root.children(&src.arena) {
        let entry = src.arena[src_child].get();
        let new_node = tree.arena.new_node(crate::types::FileEntry {
            name: entry.name.clone(),
            size: entry.size,
            file_count: entry.file_count,
            is_dir: entry.is_dir,
            extension: entry.extension.clone(),
            depth: entry.depth,
            mtime: entry.mtime,
        });
        target.append(new_node, &mut tree.arena);
        // Recurse for subdirectories
        if entry.is_dir {
            graft_children(tree, new_node, src, src_child);
        }
    }
}

/// JSON export mode (no TUI)
fn run_json_export(root_path: PathBuf, exclude: Vec<String>) -> io::Result<()> {
    let start = Instant::now();

    // Try loading from cache first
    let tree = if let Some(cached) = scanner::tree_cache::load_tree(&root_path) {
        cached
    } else {
        // No cache, run fresh scan
        let (tx, rx) = mpsc::channel();
        let handle = scanner::walk::scan_directory(root_path.clone(), tx, exclude);

        // Wait for scan to complete
        loop {
            match rx.recv_timeout(Duration::from_millis(100)) {
                Ok(ScanProgress::Done) => break,
                Ok(_) => continue,
                Err(_) => break,
            }
        }

        match handle.join() {
            Ok(Some(tree)) => tree,
            Ok(None) => {
                eprintln!("Error: Scan failed");
                std::process::exit(1);
            }
            Err(_) => {
                eprintln!("Error: Scan thread panicked");
                std::process::exit(1);
            }
        }
    };

    let scan_time_ms = start.elapsed().as_millis() as u64;

    // Compute extension stats
    let ext_stats = scanner::tree::compute_extension_stats(&tree);

    // Compute top files
    let mut top_files: Vec<(NodeId, u64)> = tree
        .root
        .descendants(&tree.arena)
        .filter(|&nid| !tree.arena[nid].get().is_dir)
        .map(|nid| (nid, tree.arena[nid].get().size))
        .collect();
    top_files.sort_by(|a, b| b.1.cmp(&a.1));
    top_files.truncate(50);

    // Find duplicates (optional, can be slow on large trees)
    let duplicates = scanner::dupes::find_duplicates(&tree);

    // Export to JSON
    let json = json_export::export_json(
        &tree,
        &ext_stats,
        &duplicates,
        &top_files,
        Some(scan_time_ms),
    );
    println!("{}", json);

    Ok(())
}
