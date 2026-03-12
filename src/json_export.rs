use serde::{Deserialize, Serialize};

use crate::types::{DuplicateGroup, ExtensionStats, FileTree};

#[derive(Serialize, Deserialize)]
pub struct JsonOutput {
    pub root: String,
    pub total_size: u64,
    pub file_count: u64,
    pub scan_time_ms: Option<u64>,
    pub top_files: Vec<JsonFile>,
    pub extensions: Vec<JsonExtension>,
    pub duplicates: Vec<JsonDuplicateGroup>,
}

#[derive(Serialize, Deserialize)]
pub struct JsonFile {
    pub path: String,
    pub size: u64,
    pub age_days: Option<u64>,
}

#[derive(Serialize, Deserialize)]
pub struct JsonExtension {
    pub ext: String,
    pub size: u64,
    pub count: u64,
}

#[derive(Serialize, Deserialize)]
pub struct JsonDuplicateGroup {
    pub hash: String,
    pub size: u64,
    pub paths: Vec<String>,
}

/// Convert tree + stats to JSON output
pub fn export_json(
    tree: &FileTree,
    ext_stats: &[ExtensionStats],
    duplicates: &[DuplicateGroup],
    top_files: &[(indextree::NodeId, u64)],
    scan_time_ms: Option<u64>,
) -> String {
    let root_entry = tree.arena[tree.root].get();

    // Convert top files
    let top_files_json: Vec<JsonFile> = top_files
        .iter()
        .map(|(node_id, size)| {
            let path = tree.full_path(*node_id);
            let entry = tree.arena[*node_id].get();
            let age_days = if entry.mtime > 0 {
                let now = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_secs();
                Some((now - entry.mtime) / 86400)
            } else {
                None
            };
            JsonFile {
                path: path.to_string_lossy().to_string(),
                size: *size,
                age_days,
            }
        })
        .collect();

    // Convert extensions
    let extensions_json: Vec<JsonExtension> = ext_stats
        .iter()
        .map(|e| JsonExtension {
            ext: e.extension.clone(),
            size: e.total_size,
            count: e.file_count,
        })
        .collect();

    // Convert duplicates
    let duplicates_json: Vec<JsonDuplicateGroup> = duplicates
        .iter()
        .map(|d| JsonDuplicateGroup {
            hash: d.hash.clone(),
            size: d.size,
            paths: d
                .paths
                .iter()
                .map(|p| p.to_string_lossy().to_string())
                .collect(),
        })
        .collect();

    let output = JsonOutput {
        root: tree.root_path.to_string_lossy().to_string(),
        total_size: root_entry.size,
        file_count: root_entry.file_count,
        scan_time_ms,
        top_files: top_files_json,
        extensions: extensions_json,
        duplicates: duplicates_json,
    };

    serde_json::to_string_pretty(&output).unwrap()
}
