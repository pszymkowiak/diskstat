use indextree::{Arena, NodeId};
use ratatui::style::Color;
use std::cell::RefCell;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;

use crate::app::SortMode;

/// Arena-based file tree for memory-efficient storage.
pub struct FileTree {
    pub arena: Arena<FileEntry>,
    pub root: NodeId,
    pub root_path: PathBuf,
    /// Cache for sorted children (composite key: (node_id, sort_mode))
    pub sorted_cache: RefCell<HashMap<(NodeId, SortMode), Vec<NodeId>>>,
}

impl FileTree {
    /// Create a new empty file tree rooted at the given path.
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
            mtime: 0,
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

    /// Get sorted children of a node according to the given sort mode.
    /// Results are cached per (node_id, mode) to avoid re-sorting.
    pub fn sorted_children_with_mode(&self, node_id: NodeId, mode: SortMode) -> Vec<NodeId> {
        let cache_key = (node_id, mode);

        // Check cache first
        {
            let cache = self.sorted_cache.borrow();
            if let Some(cached) = cache.get(&cache_key) {
                return cached.clone();
            }
        }

        // Cache miss: collect and sort children
        let mut children: Vec<NodeId> = node_id.children(&self.arena).collect();

        match mode {
            SortMode::SizeDesc => {
                children.sort_by(|a, b| {
                    let sa = self.arena[*a].get().size;
                    let sb = self.arena[*b].get().size;
                    sb.cmp(&sa)
                });
            }
            SortMode::SizeAsc => {
                children.sort_by(|a, b| {
                    let sa = self.arena[*a].get().size;
                    let sb = self.arena[*b].get().size;
                    sa.cmp(&sb)
                });
            }
            SortMode::NameAsc => {
                children.sort_by(|a, b| {
                    let na = &self.arena[*a].get().name;
                    let nb = &self.arena[*b].get().name;
                    na.cmp(nb)
                });
            }
            SortMode::NameDesc => {
                children.sort_by(|a, b| {
                    let na = &self.arena[*a].get().name;
                    let nb = &self.arena[*b].get().name;
                    nb.cmp(na)
                });
            }
            SortMode::AgeNewest => {
                children.sort_by(|a, b| {
                    let ta = self.arena[*a].get().mtime;
                    let tb = self.arena[*b].get().mtime;
                    tb.cmp(&ta) // Newest first (higher timestamp)
                });
            }
            SortMode::AgeOldest => {
                children.sort_by(|a, b| {
                    let ta = self.arena[*a].get().mtime;
                    let tb = self.arena[*b].get().mtime;
                    ta.cmp(&tb) // Oldest first (lower timestamp)
                });
            }
        }

        // Store in cache
        self.sorted_cache
            .borrow_mut()
            .insert(cache_key, children.clone());
        children
    }

    /// Get sorted children (by size, descending) of a node.
    /// Results are cached to avoid re-sorting on every call.
    /// Deprecated: Use sorted_children_with_mode instead.
    pub fn sorted_children(&self, node_id: NodeId) -> Vec<NodeId> {
        self.sorted_children_with_mode(node_id, SortMode::SizeDesc)
    }

    /// Get the total number of nodes.
    /// Get the total number of nodes in the tree.
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

    /// Remove a node from the tree and recompute parent sizes up to root.
    /// Returns the parent NodeId if it exists.
    pub fn remove_node(&mut self, node_id: NodeId) -> Option<NodeId> {
        if node_id == self.root {
            return None; // Never remove root
        }
        let parent = self.arena[node_id].parent();
        let removed_size = self.arena[node_id].get().size;
        let removed_count = if self.arena[node_id].get().is_dir {
            self.arena[node_id].get().file_count
        } else {
            1
        };

        // Detach and remove the subtree
        node_id.detach(&mut self.arena);
        // Mark all nodes in the subtree as removed
        let to_remove: Vec<NodeId> = node_id.descendants(&self.arena).collect();
        for nid in to_remove {
            nid.remove(&mut self.arena);
        }

        // Walk up and subtract sizes
        let mut current = parent;
        while let Some(nid) = current {
            let entry = self.arena[nid].get_mut();
            entry.size = entry.size.saturating_sub(removed_size);
            entry.file_count = entry.file_count.saturating_sub(removed_count);
            current = self.arena[nid].parent();
        }

        self.invalidate_sort_cache();
        parent
    }

    /// Get the modification time of the root path (used for cache validation).
    pub fn tree_mtime(&self) -> Option<std::time::SystemTime> {
        std::fs::metadata(&self.root_path).ok()?.modified().ok()
    }
}

/// A single entry in the file tree (file or directory).
pub struct FileEntry {
    pub name: String,
    pub size: u64,
    pub file_count: u64,
    pub is_dir: bool,
    pub extension: Option<Arc<str>>,
    pub depth: u16,
    pub mtime: u64, // Unix timestamp (seconds since epoch)
}

/// Statistics for files of a given extension.
#[derive(Clone)]
pub struct ExtensionStats {
    pub extension: String,
    pub total_size: u64,
    pub file_count: u64,
    pub color: Color,
}

/// A group of duplicate files with the same hash.
#[derive(Clone)]
pub struct DuplicateGroup {
    pub hash: String,
    pub size: u64,
    pub paths: Vec<PathBuf>,
}

impl DuplicateGroup {
    /// Calculate wasted space (size * (count - 1)).
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

/// Get a color for a given extension index (wraps around).
pub fn color_for_index(idx: usize) -> Color {
    EXT_COLORS[idx % EXT_COLORS.len()]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_file_tree_new() {
        let path = PathBuf::from("/tmp/test");
        let tree = FileTree::new(path.clone());
        assert_eq!(tree.root_path, path);
        assert_eq!(tree.arena.count(), 1);
        let root_entry = tree.arena[tree.root].get();
        assert!(root_entry.is_dir);
        assert_eq!(root_entry.size, 0);
    }

    #[test]
    fn test_file_tree_compute_sizes() {
        let path = PathBuf::from("/tmp/test");
        let mut tree = FileTree::new(path);

        // Add a file as child of root
        let file = tree.arena.new_node(FileEntry {
            name: "file.txt".to_string(),
            size: 1024,
            file_count: 1,
            is_dir: false,
            extension: Some("txt".into()),
            depth: 1,
            mtime: 0,
        });
        tree.root.append(file, &mut tree.arena);

        // Compute sizes
        tree.compute_sizes();

        // Root should have size of its children
        let root_entry = tree.arena[tree.root].get();
        assert_eq!(root_entry.size, 1024);
        assert_eq!(root_entry.file_count, 1);
    }

    #[test]
    fn test_extension_from_name_double_ext() {
        // This would be tested in scanner module, but we can test the concept
        let name = "file.tar.gz";
        let ext = name.rsplit('.').next().unwrap();
        assert_eq!(ext, "gz");
    }

    #[test]
    fn test_extension_from_name_dots() {
        let name = "file.name.with.dots.rs";
        let ext = name.rsplit('.').next().unwrap();
        assert_eq!(ext, "rs");
    }
}
