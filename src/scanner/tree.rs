use indextree::NodeId;
use std::collections::HashMap;

use crate::types::{color_for_index, ExtensionStats, FileTree};

/// Compute extension statistics from the file tree.
pub fn compute_extension_stats(tree: &FileTree) -> Vec<ExtensionStats> {
    let mut ext_map: HashMap<String, (u64, u64)> = HashMap::new();

    for node_id in tree.root.descendants(&tree.arena) {
        let entry = tree.arena[node_id].get();
        if !entry.is_dir {
            let ext = entry
                .extension
                .as_ref()
                .map(|s| s.to_string())
                .unwrap_or_else(|| "(no ext)".to_string());
            let stat = ext_map.entry(ext).or_insert((0, 0));
            stat.0 += entry.size;
            stat.1 += 1;
        }
    }

    let mut stats: Vec<ExtensionStats> = ext_map
        .into_iter()
        .enumerate()
        .map(
            |(i, (extension, (total_size, file_count)))| ExtensionStats {
                extension,
                total_size,
                file_count,
                color: color_for_index(i),
            },
        )
        .collect();

    stats.sort_by(|a, b| b.total_size.cmp(&a.total_size));

    // Reassign colors after sorting so top extensions get first colors
    for (i, stat) in stats.iter_mut().enumerate() {
        stat.color = color_for_index(i);
    }

    stats
}

/// Build a map from extension to color, for treemap rendering.
pub fn extension_color_map(stats: &[ExtensionStats]) -> HashMap<String, ratatui::style::Color> {
    stats
        .iter()
        .map(|s| (s.extension.clone(), s.color))
        .collect()
}

/// Get visible children for treemap rendering (filtered by minimum size).
pub fn visible_children(tree: &FileTree, node_id: NodeId, min_fraction: f64) -> Vec<NodeId> {
    let parent_size = tree.arena[node_id].get().size;
    if parent_size == 0 {
        return vec![];
    }
    let min_size = (parent_size as f64 * min_fraction) as u64;

    let mut children = tree.sorted_children(node_id);
    children.retain(|&c| tree.arena[c].get().size >= min_size.max(1));
    children
}
