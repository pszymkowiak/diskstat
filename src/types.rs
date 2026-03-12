use indextree::{Arena, NodeId};
use ratatui::style::Color;
use std::cell::RefCell;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;

/// Arena-based file tree for memory-efficient storage.
pub struct FileTree {
    pub arena: Arena<FileEntry>,
    pub root: NodeId,
    pub root_path: PathBuf,
    /// Cache for sorted children to avoid re-sorting on every call
    pub sorted_cache: RefCell<HashMap<NodeId, Vec<NodeId>>>,
}

impl FileTree {
    pub fn new(root_path: PathBuf) -> Self {
        let mut arena = Arena::new();
        let root_name = root_path
            .file_name()
            .map(|s| s.to_string_lossy().to_string())
            .unwrap_or_else(|| root_path.to_string_lossy().to_string());

        let root = arena.new_node(FileEntry {
            name: root_name,
            size: 0,
            file_count: 0,
            is_dir: true,
            extension: None,
            depth: 0,
        });

        FileTree {
            arena,
            root,
            root_path,
            sorted_cache: RefCell::new(HashMap::new()),
        }
    }

    /// Compute recursive sizes via post-order traversal.
    pub fn compute_sizes(&mut self) {
        let node_ids: Vec<NodeId> = self.root.descendants(&self.arena).collect();
        // Process in reverse (post-order: leaves first)
        for &node_id in node_ids.iter().rev() {
            let child_size: u64 = node_id
                .children(&self.arena)
                .map(|c| self.arena[c].get().size)
                .sum();
            let child_count: u64 = node_id
                .children(&self.arena)
                .map(|c| self.arena[c].get().file_count)
                .sum();

            let node = self.arena[node_id].get_mut();
            if node.is_dir {
                node.size = child_size;
                node.file_count = child_count;
            }
        }
        // Invalidate sorted_children cache after size computation
        self.invalidate_sort_cache();
    }

    /// Invalidate the sorted children cache (call after tree mutations).
    pub fn invalidate_sort_cache(&self) {
        self.sorted_cache.borrow_mut().clear();
    }

    /// Get sorted children (by size, descending) of a node.
    /// Results are cached to avoid re-sorting on every call.
    pub fn sorted_children(&self, node_id: NodeId) -> Vec<NodeId> {
        // Check cache first
        if let Some(cached) = self.sorted_cache.borrow().get(&node_id) {
            return cached.clone();
        }

        // Cache miss - compute and store
        let mut children: Vec<NodeId> = node_id.children(&self.arena).collect();
        children.sort_by(|a, b| {
            let sa = self.arena[*a].get().size;
            let sb = self.arena[*b].get().size;
            sb.cmp(&sa)
        });

        // Store in cache
        self.sorted_cache
            .borrow_mut()
            .insert(node_id, children.clone());
        children
    }

    /// Get the total number of nodes.
    pub fn node_count(&self) -> usize {
        self.arena.count()
    }

    /// Reconstruct the full path for a node by walking up ancestors.
    pub fn full_path(&self, node_id: NodeId) -> PathBuf {
        if node_id == self.root {
            return self.root_path.clone();
        }
        // Collect names from node to root
        let mut parts: Vec<&str> = Vec::new();
        let mut current = Some(node_id);
        while let Some(nid) = current {
            if nid == self.root {
                break;
            }
            parts.push(&self.arena[nid].get().name);
            current = self.arena[nid].parent();
        }
        parts.reverse();
        let mut path = self.root_path.clone();
        for part in parts {
            path.push(part);
        }
        path
    }
}

pub struct FileEntry {
    pub name: String,
    pub size: u64,
    pub file_count: u64,
    pub is_dir: bool,
    pub extension: Option<Arc<str>>,
    pub depth: u16,
}

#[derive(Clone)]
pub struct ExtensionStats {
    pub extension: String,
    pub total_size: u64,
    pub file_count: u64,
    pub color: Color,
}

#[derive(Clone)]
pub struct DuplicateGroup {
    pub hash: String,
    pub size: u64,
    pub paths: Vec<PathBuf>,
}

impl DuplicateGroup {
    pub fn wasted_size(&self) -> u64 {
        self.size * (self.paths.len() as u64 - 1)
    }
}

/// Progress update sent from scanner to UI.
pub enum ScanProgress {
    /// File/dir discovered during walk.
    Tick { file_count: u64, total_size: u64 },
    /// Scan complete.
    Done,
    /// Error during scan.
    Error(String),
}

/// Extension color palette (20 vivid colors, high contrast on dark backgrounds).
pub const EXT_COLORS: &[Color] = &[
    Color::Rgb(255, 80, 80),   // coral red
    Color::Rgb(80, 255, 120),  // vivid green
    Color::Rgb(255, 220, 50),  // bright yellow
    Color::Rgb(80, 140, 255),  // sky blue
    Color::Rgb(220, 80, 255),  // vivid magenta
    Color::Rgb(0, 230, 220),   // bright cyan
    Color::Rgb(255, 140, 50),  // bright orange
    Color::Rgb(160, 255, 80),  // lime
    Color::Rgb(255, 100, 180), // hot pink
    Color::Rgb(100, 200, 255), // light blue
    Color::Rgb(180, 130, 255), // lavender
    Color::Rgb(0, 255, 170),   // mint
    Color::Rgb(255, 180, 0),   // amber
    Color::Rgb(180, 60, 255),  // purple
    Color::Rgb(0, 200, 160),   // teal
    Color::Rgb(255, 80, 140),  // rose
    Color::Rgb(200, 160, 60),  // gold
    Color::Rgb(80, 255, 200),  // aquamarine
    Color::Rgb(120, 180, 255), // steel blue
    Color::Rgb(255, 150, 220), // orchid pink
];

pub fn color_for_index(idx: usize) -> Color {
    EXT_COLORS[idx % EXT_COLORS.len()]
}
