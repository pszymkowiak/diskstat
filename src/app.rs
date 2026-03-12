use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::mpsc;

use indextree::NodeId;
use ratatui::style::Color;

use crate::types::{DuplicateGroup, ExtensionStats, FileTree, ScanProgress};
use crate::ui::treemap::TreemapHit;

/// Text input state for the path change dialog.
pub struct PathInput {
    pub input: String,
    pub cursor: usize,
    pub completions: Vec<String>,
    pub completion_index: Option<usize>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ActiveTab {
    TreeMap,
    Extensions,
    Duplicates,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ActivePane {
    Tree,
    Map,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScanState {
    Idle,
    Scanning,
    Done,
}

pub struct App {
    pub root_path: PathBuf,
    pub tree: Option<FileTree>,
    pub scan_state: ScanState,
    pub file_count: u64,
    pub total_size: u64,

    // UI state
    pub active_tab: ActiveTab,
    pub active_pane: ActivePane,
    pub should_quit: bool,

    // Tree navigation
    pub tree_state: TreeState,

    // Treemap
    pub treemap_root: Option<NodeId>,
    pub treemap_selected: Option<NodeId>,

    // Extensions
    pub ext_stats: Vec<ExtensionStats>,
    pub ext_color_map: HashMap<String, Color>,
    pub ext_selected_index: usize,

    // Duplicates
    pub duplicates: Vec<DuplicateGroup>,
    pub dupes_selected_index: usize,
    pub dupes_scanning: bool,

    // Progress channel
    pub progress_rx: Option<mpsc::Receiver<ScanProgress>>,

    // Dialog state
    pub show_help: bool,
    pub confirm_delete: Option<(PathBuf, u64)>,
    pub path_input: Option<PathInput>,

    // Status message
    pub status_message: Option<String>,

    // Treemap hit regions (for mouse click detection)
    pub treemap_hits: Vec<TreemapHit>,

    // Disk space
    pub disk_total: u64,
    pub disk_free: u64,

    // Search (/ like vim)
    pub search_query: Option<String>,
    pub search_input: Option<String>,
    pub search_matches: Vec<NodeId>,
    pub search_index: usize,

    // Subtree rescan target
    pub subtree_target: Option<NodeId>,

    // Menu bar
    pub menu_state: MenuState,
    pub current_style_index: usize,

    // Split & treemap visibility
    pub split_pct: u16,         // 0-100, percentage of left panel (default: 40)
    pub show_treemap: bool,     // true = treemap visible, false = tree takes 100%
    pub dragging_split: bool,   // true while mouse-dragging the separator
    pub last_split_x: u16,      // x-coordinate of the separator (updated during draw)
    pub content_area: ratatui::layout::Rect, // content area (for mouse→pct conversion)

    // Rendering
    pub needs_redraw: bool,
}

pub struct MenuState {
    pub active: bool,
    pub selected_menu: usize,
    pub dropdown_open: bool,
    pub selected_item: usize,
}

pub struct TreeState {
    pub selected: Option<NodeId>,
    pub expanded: std::collections::HashSet<NodeId>,
    pub scroll_offset: usize,
    pub visible_nodes: Vec<(NodeId, u16, Vec<bool>)>, // (node_id, depth, guide: true=last child at each depth)
}

impl MenuState {
    pub fn new() -> Self {
        MenuState {
            active: false,
            selected_menu: 0,
            dropdown_open: false,
            selected_item: 0,
        }
    }
}

impl TreeState {
    pub fn new() -> Self {
        TreeState {
            selected: None,
            expanded: std::collections::HashSet::new(),
            scroll_offset: 0,
            visible_nodes: Vec::new(),
        }
    }
}

impl App {
    pub fn new(root_path: PathBuf) -> Self {
        App {
            root_path,
            tree: None,
            scan_state: ScanState::Idle,
            file_count: 0,
            total_size: 0,
            active_tab: ActiveTab::TreeMap,
            active_pane: ActivePane::Tree,
            should_quit: false,
            tree_state: TreeState::new(),
            treemap_root: None,
            treemap_selected: None,
            ext_stats: Vec::new(),
            ext_color_map: HashMap::new(),
            ext_selected_index: 0,
            duplicates: Vec::new(),
            dupes_selected_index: 0,
            dupes_scanning: false,
            progress_rx: None,
            show_help: false,
            confirm_delete: None,
            path_input: None,
            status_message: None,
            treemap_hits: Vec::new(),
            disk_total: 0,
            disk_free: 0,
            search_query: None,
            search_input: None,
            search_matches: Vec::new(),
            search_index: 0,
            subtree_target: None,
            menu_state: MenuState::new(),
            current_style_index: 0,
            split_pct: 40,
            show_treemap: true,
            dragging_split: false,
            last_split_x: 0,
            content_area: ratatui::layout::Rect::default(),
            needs_redraw: true,
        }
    }

    /// Process any pending scan progress messages.
    /// Returns true if any progress was received (needs redraw).
    pub fn poll_progress(&mut self) -> bool {
        let mut got_progress = false;
        if let Some(rx) = &self.progress_rx {
            while let Ok(msg) = rx.try_recv() {
                got_progress = true;
                match msg {
                    ScanProgress::Tick {
                        file_count,
                        total_size,
                    } => {
                        self.file_count = file_count;
                        self.total_size = total_size;
                    }
                    ScanProgress::Done => {
                        self.scan_state = ScanState::Done;
                        self.status_message = Some("Scan complete".to_string());
                    }
                    ScanProgress::Error(e) => {
                        self.status_message = Some(format!("Error: {}", e));
                    }
                }
            }
        }
        if got_progress {
            self.needs_redraw = true;
        }
        got_progress
    }

    /// Called after scan completes and tree is set.
    pub fn on_scan_complete(&mut self) {
        let (root, stats, color_map, file_count, total_size) = {
            let tree = match &self.tree {
                Some(t) => t,
                None => return,
            };
            let root = tree.root;
            let root_entry = tree.arena[root].get();
            let fc = root_entry.file_count;
            let ts = root_entry.size;
            let stats = crate::scanner::tree::compute_extension_stats(tree);
            let color_map = crate::scanner::tree::extension_color_map(&stats);
            (root, stats, color_map, fc, ts)
        };

        self.file_count = file_count;
        self.total_size = total_size;
        self.treemap_root = Some(root);
        self.tree_state.selected = Some(root);
        self.tree_state.expanded.insert(root);
        self.ext_color_map = color_map;
        self.ext_stats = stats;
        self.rebuild_visible_nodes();
        self.update_disk_space();
    }

    /// Query disk space for the root path via statvfs.
    pub fn update_disk_space(&mut self) {
        if let Some((total, free)) = get_disk_space(&self.root_path) {
            self.disk_total = total;
            self.disk_free = free;
        }
    }

    /// Rebuild the flat list of visible nodes for the tree widget.
    pub fn rebuild_visible_nodes(&mut self) {
        self.tree_state.visible_nodes.clear();
        if let Some(tree) = &self.tree {
            let root = tree.root;
            let expanded = &self.tree_state.expanded;
            let mut guide = Vec::new();
            collect_visible_into(tree, root, 0, &mut guide, expanded, &mut self.tree_state.visible_nodes);
        }
    }

    /// Navigate tree: move selection up.
    pub fn tree_up(&mut self) {
        let idx = self.selected_visible_index();
        if idx > 0 {
            let (node_id, _, _) = self.tree_state.visible_nodes[idx - 1];
            self.tree_state.selected = Some(node_id);
            self.sync_treemap_selection();
            // Scroll if needed
            if idx - 1 < self.tree_state.scroll_offset {
                self.tree_state.scroll_offset = idx - 1;
            }
        }
    }

    /// Navigate tree: move selection down.
    pub fn tree_down(&mut self) {
        let idx = self.selected_visible_index();
        if idx + 1 < self.tree_state.visible_nodes.len() {
            let (node_id, _, _) = self.tree_state.visible_nodes[idx + 1];
            self.tree_state.selected = Some(node_id);
            self.sync_treemap_selection();
        }
    }

    /// Expand selected node (right arrow).
    pub fn tree_expand(&mut self) {
        if let Some(selected) = self.tree_state.selected {
            if let Some(tree) = &self.tree {
                if tree.arena[selected].get().is_dir {
                    self.tree_state.expanded.insert(selected);
                    self.rebuild_visible_nodes();
                }
            }
        }
    }

    /// Collapse selected node (left arrow).
    pub fn tree_collapse(&mut self) {
        if let Some(selected) = self.tree_state.selected {
            if self.tree_state.expanded.contains(&selected) {
                self.tree_state.expanded.remove(&selected);
                self.rebuild_visible_nodes();
            } else {
                // Go to parent
                if let Some(tree) = &self.tree {
                    if let Some(parent) = tree.arena[selected].parent() {
                        self.tree_state.selected = Some(parent);
                        self.sync_treemap_selection();
                    }
                }
            }
        }
    }

    /// Get the index of the currently selected node in visible_nodes.
    fn selected_visible_index(&self) -> usize {
        if let Some(selected) = self.tree_state.selected {
            self.tree_state
                .visible_nodes
                .iter()
                .position(|(id, _, _)| *id == selected)
                .unwrap_or(0)
        } else {
            0
        }
    }

    /// Sync treemap selection with tree selection.
    fn sync_treemap_selection(&mut self) {
        self.treemap_selected = self.tree_state.selected;
    }

    /// Enter selected treemap node (zoom in).
    pub fn treemap_enter(&mut self) {
        if let Some(selected) = self.treemap_selected {
            if let Some(tree) = &self.tree {
                if tree.arena[selected].get().is_dir {
                    self.treemap_root = Some(selected);
                }
            }
        }
    }

    /// Go back to parent in treemap (zoom out).
    pub fn treemap_back(&mut self) {
        if let Some(root) = self.treemap_root {
            if let Some(tree) = &self.tree {
                if let Some(parent) = tree.arena[root].parent() {
                    self.treemap_root = Some(parent);
                }
            }
        }
    }

    /// Ensure scroll keeps selected item visible.
    pub fn ensure_visible(&mut self, visible_height: usize) {
        let idx = self.selected_visible_index();
        if idx < self.tree_state.scroll_offset {
            self.tree_state.scroll_offset = idx;
        } else if idx >= self.tree_state.scroll_offset + visible_height {
            self.tree_state.scroll_offset = idx - visible_height + 1;
        }
    }

    /// Get the path of the currently selected node.
    pub fn selected_path(&self) -> Option<PathBuf> {
        let selected = self.tree_state.selected?;
        let tree = self.tree.as_ref()?;
        Some(tree.full_path(selected))
    }

    /// Open the path input dialog pre-filled with the current root.
    pub fn open_path_input(&mut self) {
        self.path_input = Some(PathInput::new(
            self.root_path.to_string_lossy().to_string(),
        ));
    }

    /// Expand the tree so a given node becomes visible and selected.
    pub fn expand_to_node(&mut self, target: NodeId) {
        if let Some(tree) = &self.tree {
            // Walk up from target to root, expanding each ancestor
            let mut current = Some(target);
            let mut ancestors = Vec::new();
            while let Some(nid) = current {
                if nid == tree.root {
                    ancestors.push(nid);
                    break;
                }
                ancestors.push(nid);
                current = tree.arena[nid].parent();
            }
            for &anc in &ancestors {
                if tree.arena[anc].get().is_dir {
                    self.tree_state.expanded.insert(anc);
                }
            }
            self.tree_state.selected = Some(target);
            self.treemap_selected = Some(target);
            self.rebuild_visible_nodes();
        }
    }

    /// Perform search: find all nodes whose name contains the query (case-insensitive).
    pub fn search_execute(&mut self) {
        let query = match &self.search_query {
            Some(q) if !q.is_empty() => q.to_lowercase(),
            _ => return,
        };
        self.search_matches.clear();
        self.search_index = 0;
        if let Some(tree) = &self.tree {
            for nid in tree.root.descendants(&tree.arena) {
                let name = &tree.arena[nid].get().name;
                if name.to_lowercase().contains(&query) {
                    self.search_matches.push(nid);
                }
            }
        }
        if !self.search_matches.is_empty() {
            let target = self.search_matches[0];
            self.expand_to_node(target);
            self.status_message = Some(format!(
                "Match 1/{}", self.search_matches.len()
            ));
        } else {
            self.status_message = Some("No matches found".to_string());
        }
    }

    /// Jump to next search match.
    pub fn search_next(&mut self) {
        if self.search_matches.is_empty() {
            return;
        }
        self.search_index = (self.search_index + 1) % self.search_matches.len();
        let target = self.search_matches[self.search_index];
        self.expand_to_node(target);
        self.status_message = Some(format!(
            "Match {}/{}",
            self.search_index + 1,
            self.search_matches.len()
        ));
    }

    /// Jump to previous search match.
    pub fn search_prev(&mut self) {
        if self.search_matches.is_empty() {
            return;
        }
        if self.search_index == 0 {
            self.search_index = self.search_matches.len() - 1;
        } else {
            self.search_index -= 1;
        }
        let target = self.search_matches[self.search_index];
        self.expand_to_node(target);
        self.status_message = Some(format!(
            "Match {}/{}",
            self.search_index + 1,
            self.search_matches.len()
        ));
    }

    /// Reset all state for a new scan of a different directory.
    pub fn reset_for_scan(&mut self, new_path: PathBuf) {
        self.root_path = new_path;
        self.tree = None;
        self.scan_state = ScanState::Idle;
        self.file_count = 0;
        self.total_size = 0;
        self.tree_state = TreeState::new();
        self.treemap_root = None;
        self.treemap_selected = None;
        self.ext_stats.clear();
        self.ext_color_map.clear();
        self.ext_selected_index = 0;
        self.duplicates.clear();
        self.dupes_selected_index = 0;
        self.dupes_scanning = false;
        self.progress_rx = None;
        self.status_message = None;
        self.active_tab = ActiveTab::TreeMap;
        self.treemap_hits.clear();
        self.disk_total = 0;
        self.disk_free = 0;
        self.search_query = None;
        self.search_input = None;
        self.search_matches.clear();
        self.search_index = 0;
        self.subtree_target = None;
        self.menu_state = MenuState::new();
        // Note: current_style_index, split_pct, show_treemap are intentionally preserved across rescans
    }
}

impl PathInput {
    pub fn new(initial: String) -> Self {
        let cursor = initial.len();
        PathInput {
            input: initial,
            cursor,
            completions: Vec::new(),
            completion_index: None,
        }
    }

    pub fn insert_char(&mut self, c: char) {
        self.input.insert(self.cursor, c);
        self.cursor += c.len_utf8();
        self.completion_index = None;
    }

    pub fn backspace(&mut self) {
        if self.cursor > 0 {
            let prev = self.input[..self.cursor]
                .char_indices()
                .next_back()
                .map(|(i, _)| i)
                .unwrap_or(0);
            self.input.drain(prev..self.cursor);
            self.cursor = prev;
            self.completion_index = None;
        }
    }

    pub fn delete(&mut self) {
        if self.cursor < self.input.len() {
            let next = self.input[self.cursor..]
                .char_indices()
                .nth(1)
                .map(|(i, _)| self.cursor + i)
                .unwrap_or(self.input.len());
            self.input.drain(self.cursor..next);
            self.completion_index = None;
        }
    }

    pub fn move_left(&mut self) {
        if self.cursor > 0 {
            self.cursor = self.input[..self.cursor]
                .char_indices()
                .next_back()
                .map(|(i, _)| i)
                .unwrap_or(0);
        }
    }

    pub fn move_right(&mut self) {
        if self.cursor < self.input.len() {
            self.cursor = self.input[self.cursor..]
                .char_indices()
                .nth(1)
                .map(|(i, _)| self.cursor + i)
                .unwrap_or(self.input.len());
        }
    }

    pub fn move_home(&mut self) {
        self.cursor = 0;
    }

    pub fn move_end(&mut self) {
        self.cursor = self.input.len();
    }

    /// Tab-complete the current input path.
    pub fn complete(&mut self) {
        // If we already have completions, cycle through them
        if !self.completions.is_empty() {
            let idx = match self.completion_index {
                Some(i) => (i + 1) % self.completions.len(),
                None => 0,
            };
            self.completion_index = Some(idx);
            self.input = self.completions[idx].clone();
            self.cursor = self.input.len();
            return;
        }

        // Build completions from filesystem
        let path = PathBuf::from(&self.input);
        let (dir, prefix) = if self.input.ends_with('/') || self.input.ends_with(std::path::MAIN_SEPARATOR) {
            (path.clone(), String::new())
        } else {
            let parent = path.parent().unwrap_or_else(|| std::path::Path::new("/"));
            let prefix = path
                .file_name()
                .map(|s| s.to_string_lossy().to_string())
                .unwrap_or_default();
            (parent.to_path_buf(), prefix)
        };

        if let Ok(entries) = std::fs::read_dir(&dir) {
            let mut matches: Vec<String> = Vec::new();
            for entry in entries.flatten() {
                let name = entry.file_name().to_string_lossy().to_string();
                if name.starts_with(&prefix) {
                    let full = dir.join(&name);
                    if full.is_dir() {
                        matches.push(format!("{}/", full.display()));
                    } else {
                        matches.push(full.to_string_lossy().to_string());
                    }
                }
            }
            matches.sort();
            if matches.len() == 1 {
                self.input = matches[0].clone();
                self.cursor = self.input.len();
                self.completions.clear();
                self.completion_index = None;
            } else if !matches.is_empty() {
                self.completions = matches;
                self.completion_index = Some(0);
                self.input = self.completions[0].clone();
                self.cursor = self.input.len();
            }
        }
    }

    /// Validate and return the path, or None if invalid.
    pub fn validate(&self) -> Option<PathBuf> {
        let path = PathBuf::from(self.input.trim_end_matches('/'));
        let path = path.canonicalize().ok()?;
        if path.is_dir() {
            Some(path)
        } else {
            None
        }
    }
}

fn collect_visible_into(
    tree: &FileTree,
    node_id: NodeId,
    depth: u16,
    guide: &mut Vec<bool>,
    expanded: &std::collections::HashSet<NodeId>,
    result: &mut Vec<(NodeId, u16, Vec<bool>)>,
) {
    // Clone guide for storage in result, but reuse the same Vec for recursion
    result.push((node_id, depth, guide.clone()));
    if expanded.contains(&node_id) {
        let children = tree.sorted_children(node_id);
        let count = children.len();
        for (i, child) in children.into_iter().enumerate() {
            let is_last = i == count - 1;
            guide.push(is_last);
            collect_visible_into(tree, child, depth + 1, guide, expanded, result);
            guide.pop();
        }
    }
}

/// Get total and free disk space for a path using statvfs.
fn get_disk_space(path: &std::path::Path) -> Option<(u64, u64)> {
    use std::ffi::CString;
    use std::mem::MaybeUninit;
    use std::os::unix::ffi::OsStrExt;

    let c_path = CString::new(path.as_os_str().as_bytes()).ok()?;
    unsafe {
        let mut stat = MaybeUninit::<libc::statvfs>::uninit();
        if libc::statvfs(c_path.as_ptr(), stat.as_mut_ptr()) == 0 {
            let stat = stat.assume_init();
            let total = stat.f_blocks as u64 * stat.f_frsize;
            let free = stat.f_bavail as u64 * stat.f_frsize;
            Some((total, free))
        } else {
            None
        }
    }
}
