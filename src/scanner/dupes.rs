use std::collections::HashMap;
use std::fs::File;
use std::io::Read;
use std::path::PathBuf;

use rayon::prelude::*;

use crate::types::{DuplicateGroup, FileTree};

const PARTIAL_HASH_SIZE: usize = 4096;

/// Detect duplicate files using a 3-pass strategy:
/// 1. Group by size
/// 2. Partial hash (first 4KB)
/// 3. Full hash (only if partial collision)
pub fn find_duplicates(tree: &FileTree) -> Vec<DuplicateGroup> {
    // Pass 1: group files by size
    let mut size_groups: HashMap<u64, Vec<PathBuf>> = HashMap::new();

    for node_id in tree.root.descendants(&tree.arena) {
        let entry = tree.arena[node_id].get();
        if !entry.is_dir && entry.size > 0 {
            let path = tree.full_path(node_id);
            size_groups
                .entry(entry.size)
                .or_default()
                .push(path);
        }
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
    let mut file = File::open(path).ok()?;
    let mut buf = vec![0u8; PARTIAL_HASH_SIZE];
    let n = file.read(&mut buf).ok()?;
    buf.truncate(n);
    Some(blake3::hash(&buf).to_hex().to_string())
}

fn full_hash(path: &PathBuf) -> Option<String> {
    let mut file = File::open(path).ok()?;
    let mut hasher = blake3::Hasher::new();
    let mut buf = vec![0u8; 65536];
    loop {
        let n = file.read(&mut buf).ok()?;
        if n == 0 {
            break;
        }
        hasher.update(&buf[..n]);
    }
    Some(hasher.finalize().to_hex().to_string())
}
