use std::cell::RefCell;
use std::collections::HashMap;
use std::fs::File;
use std::io::Read;
use std::path::PathBuf;

use rayon::prelude::*;

use crate::types::{DuplicateGroup, FileTree};

/// Thread-safe representation of files for duplicate scanning
pub struct FileSnapshot {
    pub files: Vec<(PathBuf, u64)>, // (path, size)
}

impl FileSnapshot {
    pub fn from_tree(tree: &FileTree) -> Self {
        let files: Vec<(PathBuf, u64)> = tree
            .root
            .descendants(&tree.arena)
            .filter_map(|node_id| {
                let entry = tree.arena[node_id].get();
                if !entry.is_dir && entry.size > 0 {
                    Some((tree.full_path(node_id), entry.size))
                } else {
                    None
                }
            })
            .collect();
        FileSnapshot { files }
    }
}

const PARTIAL_HASH_SIZE: usize = 4096;
const FULL_HASH_BUF_SIZE: usize = 65536;

thread_local! {
    static PARTIAL_BUF: RefCell<Vec<u8>> = RefCell::new(vec![0u8; PARTIAL_HASH_SIZE]);
    static FULL_BUF: RefCell<Vec<u8>> = RefCell::new(vec![0u8; FULL_HASH_BUF_SIZE]);
}

/// Detect duplicate files using a 3-pass strategy:
/// 1. Group by size
/// 2. Partial hash (first 4KB)
/// 3. Full hash (only if partial collision)
pub fn find_duplicates(tree: &FileTree) -> Vec<DuplicateGroup> {
    let snapshot = FileSnapshot::from_tree(tree);
    find_duplicates_from_snapshot(&snapshot)
}

/// Find duplicates from a thread-safe snapshot (can be used in background threads)
pub fn find_duplicates_from_snapshot(snapshot: &FileSnapshot) -> Vec<DuplicateGroup> {
    // Pass 1: group files by size
    let mut size_groups: HashMap<u64, Vec<PathBuf>> = HashMap::new();

    for (path, size) in &snapshot.files {
        size_groups.entry(*size).or_default().push(path.clone());
    }

    // Keep only groups with 2+ files
    let candidates: Vec<(u64, Vec<PathBuf>)> = size_groups
        .into_iter()
        .filter(|(_, paths)| paths.len() >= 2)
        .collect();

    // Pass 2: partial hash
    let partial_groups: Vec<(u64, Vec<PathBuf>)> = candidates
        .into_par_iter()
        .flat_map(|(size, paths)| {
            let mut hash_groups: HashMap<String, Vec<PathBuf>> = HashMap::new();
            for path in &paths {
                if let Some(hash) = partial_hash(path) {
                    hash_groups.entry(hash).or_default().push(path.clone());
                }
            }
            hash_groups
                .into_iter()
                .filter(|(_, p)| p.len() >= 2)
                .map(move |(_, p)| (size, p))
                .collect::<Vec<_>>()
        })
        .collect();

    // Pass 3: full hash
    let duplicate_groups: Vec<DuplicateGroup> = partial_groups
        .into_par_iter()
        .flat_map(|(size, paths)| {
            let mut hash_groups: HashMap<String, Vec<PathBuf>> = HashMap::new();
            for path in &paths {
                if let Some(hash) = full_hash(path) {
                    hash_groups.entry(hash).or_default().push(path.clone());
                }
            }
            hash_groups
                .into_iter()
                .filter(|(_, p)| p.len() >= 2)
                .map(move |(hash, paths)| DuplicateGroup { hash, size, paths })
                .collect::<Vec<_>>()
        })
        .collect();

    let mut result = duplicate_groups;
    result.sort_by_key(|a| std::cmp::Reverse(a.wasted_size()));
    result
}

fn partial_hash(path: &PathBuf) -> Option<String> {
    PARTIAL_BUF.with(|buf_cell| {
        let mut buf = buf_cell.borrow_mut();
        let mut file = File::open(path).ok()?;
        let n = file.read(&mut buf).ok()?;
        Some(blake3::hash(&buf[..n]).to_hex().to_string())
    })
}

fn full_hash(path: &PathBuf) -> Option<String> {
    FULL_BUF.with(|buf_cell| {
        let mut buf = buf_cell.borrow_mut();
        let mut file = File::open(path).ok()?;
        let mut hasher = blake3::Hasher::new();
        loop {
            let n = file.read(&mut buf).ok()?;
            if n == 0 {
                break;
            }
            hasher.update(&buf[..n]);
        }
        Some(hasher.finalize().to_hex().to_string())
    })
}
