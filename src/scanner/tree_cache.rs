use std::collections::HashMap;
use std::io::{self, BufReader, BufWriter, Read, Write};
use std::path::{Path, PathBuf};

use indextree::NodeId;

use crate::types::{FileEntry, FileTree};

const MAGIC: &[u8; 4] = b"DST1";

/// Save a completed FileTree to a compact binary cache file.
pub fn save_tree(tree: &FileTree) -> io::Result<()> {
    let cache_path = cache_path_for(&tree.root_path);
    let _ = std::fs::create_dir_all(cache_path.parent().unwrap());

    let file = std::fs::File::create(&cache_path)?;
    let mut w = BufWriter::with_capacity(256 * 1024, file);

    w.write_all(MAGIC)?;

    // Root path
    let root_bytes = tree.root_path.to_string_lossy().into_owned().into_bytes();
    write_u32(&mut w, root_bytes.len() as u32)?;
    w.write_all(&root_bytes)?;

    // DFS order + index map
    let dfs_order: Vec<NodeId> = tree.root.descendants(&tree.arena).collect();
    let mut id_to_index: HashMap<NodeId, i32> = HashMap::with_capacity(dfs_order.len());
    for (i, &nid) in dfs_order.iter().enumerate() {
        id_to_index.insert(nid, i as i32);
    }

    write_u32(&mut w, dfs_order.len() as u32)?;

    for &nid in &dfs_order {
        let entry = tree.arena[nid].get();

        let parent_idx = tree.arena[nid]
            .parent()
            .and_then(|p| id_to_index.get(&p).copied())
            .unwrap_or(-1);
        write_i32(&mut w, parent_idx)?;

        let name_bytes = entry.name.as_bytes();
        write_u16(&mut w, name_bytes.len() as u16)?;
        w.write_all(name_bytes)?;

        write_u64(&mut w, entry.size)?;
        write_u64(&mut w, entry.file_count)?;
        w.write_all(&[entry.is_dir as u8])?;

        if let Some(ref ext) = entry.extension {
            let ext_bytes = ext.as_bytes();
            write_u16(&mut w, ext_bytes.len() as u16)?;
            w.write_all(ext_bytes)?;
        } else {
            write_u16(&mut w, 0)?;
        }

        write_u16(&mut w, entry.depth)?;
    }

    w.flush()?;
    Ok(())
}

/// Load a FileTree from the binary cache. Returns None if cache is missing or invalid.
pub fn load_tree(root_path: &Path) -> Option<FileTree> {
    let cache_path = cache_path_for(root_path);
    let file = std::fs::File::open(&cache_path).ok()?;
    let mut r = BufReader::with_capacity(256 * 1024, file);

    let mut magic = [0u8; 4];
    r.read_exact(&mut magic).ok()?;
    if &magic != MAGIC {
        return None;
    }

    let path_len = read_u32(&mut r).ok()? as usize;
    let mut path_buf = vec![0u8; path_len];
    r.read_exact(&mut path_buf).ok()?;
    let stored_root = String::from_utf8_lossy(&path_buf);
    if stored_root != root_path.to_string_lossy() {
        return None;
    }

    let node_count = read_u32(&mut r).ok()? as usize;
    if node_count == 0 {
        return None;
    }

    let mut arena = indextree::Arena::with_capacity(node_count);
    let mut node_ids: Vec<NodeId> = Vec::with_capacity(node_count);
    let mut root_id = None;

    for _ in 0..node_count {
        let parent_idx = read_i32(&mut r).ok()?;

        let name_len = read_u16(&mut r).ok()? as usize;
        let mut name_buf = vec![0u8; name_len];
        r.read_exact(&mut name_buf).ok()?;
        let name = String::from_utf8(name_buf).unwrap_or_default();

        let size = read_u64(&mut r).ok()?;
        let file_count = read_u64(&mut r).ok()?;

        let mut is_dir_byte = [0u8; 1];
        r.read_exact(&mut is_dir_byte).ok()?;
        let is_dir = is_dir_byte[0] != 0;

        let ext_len = read_u16(&mut r).ok()? as usize;
        let extension = if ext_len > 0 {
            let mut ext_buf = vec![0u8; ext_len];
            r.read_exact(&mut ext_buf).ok()?;
            Some(String::from_utf8(ext_buf).unwrap_or_default())
        } else {
            None
        };

        let depth = read_u16(&mut r).ok()?;

        let nid = arena.new_node(FileEntry {
            name,
            size,
            file_count,
            is_dir,
            extension,
            depth,
        });

        if parent_idx < 0 {
            root_id = Some(nid);
        } else if let Some(&parent_nid) = node_ids.get(parent_idx as usize) {
            parent_nid.append(nid, &mut arena);
        }

        node_ids.push(nid);
    }

    Some(FileTree {
        arena,
        root: root_id?,
        root_path: root_path.to_path_buf(),
    })
}

/// Delete the binary cache for a given root path.
pub fn invalidate(root_path: &Path) {
    let cache_path = cache_path_for(root_path);
    let _ = std::fs::remove_file(cache_path);
}

fn cache_path_for(root_path: &Path) -> PathBuf {
    let cache_dir = if let Some(home) = std::env::var_os("HOME") {
        PathBuf::from(home).join(".cache").join("diskstat")
    } else {
        PathBuf::from("/tmp").join("diskstat")
    };
    let hash = blake3::hash(root_path.to_string_lossy().as_bytes());
    cache_dir.join(format!("{}.bin", &hash.to_hex()[..16]))
}

fn write_u16(w: &mut impl Write, v: u16) -> io::Result<()> {
    w.write_all(&v.to_le_bytes())
}
fn write_u32(w: &mut impl Write, v: u32) -> io::Result<()> {
    w.write_all(&v.to_le_bytes())
}
fn write_i32(w: &mut impl Write, v: i32) -> io::Result<()> {
    w.write_all(&v.to_le_bytes())
}
fn write_u64(w: &mut impl Write, v: u64) -> io::Result<()> {
    w.write_all(&v.to_le_bytes())
}

fn read_u16(r: &mut impl Read) -> io::Result<u16> {
    let mut buf = [0u8; 2];
    r.read_exact(&mut buf)?;
    Ok(u16::from_le_bytes(buf))
}
fn read_u32(r: &mut impl Read) -> io::Result<u32> {
    let mut buf = [0u8; 4];
    r.read_exact(&mut buf)?;
    Ok(u32::from_le_bytes(buf))
}
fn read_i32(r: &mut impl Read) -> io::Result<i32> {
    let mut buf = [0u8; 4];
    r.read_exact(&mut buf)?;
    Ok(i32::from_le_bytes(buf))
}
fn read_u64(r: &mut impl Read) -> io::Result<u64> {
    let mut buf = [0u8; 8];
    r.read_exact(&mut buf)?;
    Ok(u64::from_le_bytes(buf))
}
