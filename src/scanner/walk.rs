use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex, mpsc};
use std::thread;

#[cfg(unix)]
use std::os::unix::fs::MetadataExt;

use indextree::NodeId;
use rayon::prelude::*;

use crate::scanner::cache::ScanCache;
use crate::types::{FileEntry, FileTree, ScanProgress};

/// Directory child returned by platform-specific read.
#[derive(Clone)]
pub(crate) struct DirChild {
    pub(crate) name: String,
    pub(crate) is_dir: bool,
    pub(crate) size: u64,
}

/// Pending cache write.
struct PendingWrite {
    dir_path: PathBuf,
    children: Vec<DirChild>,
}

/// Shared context for parallel scan.
struct ScanContext {
    arena: Mutex<indextree::Arena<FileEntry>>,
    file_count: AtomicU64,
    total_size: AtomicU64,
    /// Read-only cache index (loaded at startup, no lock needed).
    cache: Option<ScanCache>,
    /// Pending cache writes (locked only for push).
    pending: Mutex<Vec<PendingWrite>>,
}

pub fn scan_directory(
    root: PathBuf,
    progress_tx: mpsc::Sender<ScanProgress>,
) -> thread::JoinHandle<Option<FileTree>> {
    thread::spawn(move || {
        let root_name = root
            .file_name()
            .map(|s| s.to_string_lossy().to_string())
            .unwrap_or_else(|| root.to_string_lossy().to_string());

        let mut arena = indextree::Arena::new();
        let root_id = arena.new_node(FileEntry {
            name: root_name,
            size: 0,
            file_count: 0,
            is_dir: true,
            extension: None,
            depth: 0,
        });

        let cache = ScanCache::open(&root).ok();

        let ctx = Arc::new(ScanContext {
            arena: Mutex::new(arena),
            file_count: AtomicU64::new(0),
            total_size: AtomicU64::new(0),
            cache,
            pending: Mutex::new(Vec::new()),
        });

        // Progress reporter
        let ctx2 = Arc::clone(&ctx);
        let ptx = progress_tx.clone();
        let (stop_tx, stop_rx) = mpsc::channel::<()>();

        let progress_thread = thread::spawn(move || {
            loop {
                match stop_rx.try_recv() {
                    Ok(()) | Err(mpsc::TryRecvError::Disconnected) => break,
                    Err(mpsc::TryRecvError::Empty) => {}
                }
                let _ = ptx.send(ScanProgress::Tick {
                    file_count: ctx2.file_count.load(Ordering::Relaxed),
                    total_size: ctx2.total_size.load(Ordering::Relaxed),
                });
                thread::sleep(std::time::Duration::from_millis(80));
            }
        });

        // Parallel recursive scan
        scan_dir_recursive(&ctx, &root, root_id, 1);

        // Stop progress
        let _ = stop_tx.send(());
        let _ = progress_thread.join();

        // Flush cache writes to SQLite
        let mut ctx = match Arc::try_unwrap(ctx) {
            Ok(c) => c,
            Err(_) => return None,
        };

        if let Some(ref mut cache) = ctx.cache {
            let pending = ctx.pending.into_inner().unwrap_or_default();
            for pw in &pending {
                cache.queue_store(&pw.dir_path, &pw.children);
            }
            let _ = cache.flush();
        }

        let arena = ctx.arena.into_inner().ok()?;
        let mut tree = FileTree {
            arena,
            root: root_id,
            root_path: root,
        };

        tree.compute_sizes();

        let _ = progress_tx.send(ScanProgress::Tick {
            file_count: tree.arena[tree.root].get().file_count,
            total_size: tree.arena[tree.root].get().size,
        });
        let _ = progress_tx.send(ScanProgress::Done);

        Some(tree)
    })
}

fn scan_dir_recursive(ctx: &ScanContext, dir_path: &Path, parent_id: NodeId, depth: u16) {
    // Try cache (lock-free read from in-memory HashMap)
    let children = if let Some(cache) = &ctx.cache {
        if let Some(cached) = cache.lookup_dir(dir_path) {
            cached
        } else {
            read_and_cache(ctx, dir_path)
        }
    } else {
        match read_dir_entries(dir_path) {
            Ok(c) => c,
            Err(_) => return,
        }
    };

    let mut subdirs: Vec<(PathBuf, NodeId)> = Vec::new();

    {
        let mut arena = ctx.arena.lock().unwrap();
        for child in &children {
            let extension = if !child.is_dir {
                extension_from_name(&child.name)
            } else {
                None
            };

            let node_id = arena.new_node(FileEntry {
                name: child.name.clone(),
                size: child.size,
                file_count: if child.is_dir { 0 } else { 1 },
                is_dir: child.is_dir,
                extension,
                depth,
            });
            parent_id.append(node_id, &mut arena);

            if child.is_dir {
                subdirs.push((dir_path.join(&child.name), node_id));
            } else {
                ctx.file_count.fetch_add(1, Ordering::Relaxed);
                ctx.total_size.fetch_add(child.size, Ordering::Relaxed);
            }
        }
    }

    // Recurse in parallel via rayon work-stealing
    subdirs.par_iter().for_each(|(path, node_id)| {
        scan_dir_recursive(ctx, path, *node_id, depth.saturating_add(1));
    });
}

/// Read directory from filesystem and queue it for cache storage.
fn read_and_cache(ctx: &ScanContext, dir_path: &Path) -> Vec<DirChild> {
    match read_dir_entries(dir_path) {
        Ok(c) => {
            if ctx.cache.is_some() {
                if let Ok(mut pending) = ctx.pending.lock() {
                    pending.push(PendingWrite {
                        dir_path: dir_path.to_path_buf(),
                        children: c.clone(),
                    });
                }
            }
            c
        }
        Err(_) => Vec::new(),
    }
}

fn extension_from_name(name: &str) -> Option<String> {
    let dot_pos = name.rfind('.')?;
    if dot_pos == 0 || dot_pos == name.len() - 1 {
        return None;
    }
    Some(name[dot_pos + 1..].to_lowercase())
}

// ── Platform-specific directory reading ──────────────────────────────────────

#[cfg(target_os = "macos")]
fn read_dir_entries(path: &Path) -> Result<Vec<DirChild>, std::io::Error> {
    match read_dir_bulk_macos(path) {
        Ok(entries) => Ok(entries),
        Err(_) => read_dir_fallback(path),
    }
}

#[cfg(not(target_os = "macos"))]
fn read_dir_entries(path: &Path) -> Result<Vec<DirChild>, std::io::Error> {
    read_dir_fallback(path)
}

fn read_dir_fallback(path: &Path) -> Result<Vec<DirChild>, std::io::Error> {
    let mut children = Vec::new();
    for entry in std::fs::read_dir(path)? {
        let entry = match entry {
            Ok(e) => e,
            Err(_) => continue,
        };
        let ft = match entry.file_type() {
            Ok(ft) => ft,
            Err(_) => continue,
        };
        let is_dir = ft.is_dir();
        let size = if is_dir {
            0
        } else {
            entry.metadata().map(|m| {
                #[cfg(unix)]
                { m.blocks() * 512 }
                #[cfg(not(unix))]
                { m.len() }
            }).unwrap_or(0)
        };
        let name = entry.file_name().to_string_lossy().to_string();
        children.push(DirChild { name, is_dir, size });
    }
    Ok(children)
}

// ── macOS getattrlistbulk ────────────────────────────────────────────────────

#[cfg(target_os = "macos")]
fn read_dir_bulk_macos(path: &Path) -> Result<Vec<DirChild>, std::io::Error> {
    use libc::{c_int, c_void};
    use std::ffi::CString;
    use std::os::unix::ffi::OsStrExt;

    const ATTR_BIT_MAP_COUNT: u16 = 5;
    const ATTR_CMN_RETURNED_ATTRS: u32 = 0x80000000;
    const ATTR_CMN_NAME: u32 = 0x00000001;
    const ATTR_CMN_OBJTYPE: u32 = 0x00000008;
    const ATTR_FILE_ALLOCSIZE: u32 = 0x00000004;
    const VDIR: u32 = 2;

    #[repr(C)]
    struct AttrList {
        bitmapcount: u16,
        reserved: u16,
        commonattr: u32,
        volattr: u32,
        dirattr: u32,
        fileattr: u32,
        forkattr: u32,
    }

    extern "C" {
        fn getattrlistbulk(
            dirfd: c_int,
            alist: *mut AttrList,
            attribute_buffer: *mut c_void,
            buffer_size: usize,
            options: u64,
        ) -> c_int;
    }

    let c_path = CString::new(path.as_os_str().as_bytes())
        .map_err(|_| std::io::Error::new(std::io::ErrorKind::InvalidInput, "invalid path"))?;

    let fd = unsafe { libc::open(c_path.as_ptr(), libc::O_RDONLY) };
    if fd < 0 {
        return Err(std::io::Error::last_os_error());
    }

    struct FdGuard(c_int);
    impl Drop for FdGuard {
        fn drop(&mut self) {
            unsafe { libc::close(self.0); }
        }
    }
    let _guard = FdGuard(fd);

    let mut alist = AttrList {
        bitmapcount: ATTR_BIT_MAP_COUNT,
        reserved: 0,
        commonattr: ATTR_CMN_RETURNED_ATTRS | ATTR_CMN_NAME | ATTR_CMN_OBJTYPE,
        volattr: 0,
        dirattr: 0,
        fileattr: ATTR_FILE_ALLOCSIZE,
        forkattr: 0,
    };

    let buf_size: usize = 256 * 1024;
    let mut buffer = vec![0u8; buf_size];
    let mut results = Vec::new();

    loop {
        let count = unsafe {
            getattrlistbulk(
                fd,
                &mut alist,
                buffer.as_mut_ptr() as *mut c_void,
                buf_size,
                0,
            )
        };

        if count < 0 {
            return Err(std::io::Error::last_os_error());
        }
        if count == 0 {
            break;
        }

        let mut offset = 0usize;
        for _ in 0..count {
            if offset + 4 > buf_size {
                break;
            }

            let length = read_u32(&buffer, offset) as usize;
            if length == 0 || offset + length > buf_size {
                break;
            }
            let record_end = offset + length;
            offset += 4;

            if offset + 20 > record_end {
                offset = record_end;
                continue;
            }
            let returned_file = read_u32(&buffer, offset + 12);
            offset += 20;

            if offset + 8 > record_end {
                offset = record_end;
                continue;
            }
            let name_ref_start = offset;
            let name_data_off = read_i32(&buffer, offset);
            let name_len = read_u32(&buffer, offset + 4) as usize;
            offset += 8;

            if offset + 4 > record_end {
                offset = record_end;
                continue;
            }
            let obj_type = read_u32(&buffer, offset);
            offset += 4;

            let size = if returned_file & ATTR_FILE_ALLOCSIZE != 0 {
                if offset + 8 <= record_end {
                    read_i64(&buffer, offset).max(0) as u64
                } else {
                    0
                }
            } else {
                0
            };

            let name_abs = (name_ref_start as isize + name_data_off as isize) as usize;
            let name = if name_abs + name_len <= buf_size && name_len > 0 {
                let name_bytes = &buffer[name_abs..name_abs + name_len];
                let end = name_bytes.iter().position(|&b| b == 0).unwrap_or(name_len);
                String::from_utf8_lossy(&name_bytes[..end]).to_string()
            } else {
                offset = record_end;
                continue;
            };

            if name == "." || name == ".." {
                offset = record_end;
                continue;
            }

            results.push(DirChild {
                name,
                is_dir: obj_type == VDIR,
                size: if obj_type == VDIR { 0 } else { size },
            });

            offset = record_end;
        }
    }

    Ok(results)
}

#[cfg(target_os = "macos")]
#[inline]
fn read_u32(buf: &[u8], off: usize) -> u32 {
    u32::from_ne_bytes(buf[off..off + 4].try_into().unwrap())
}

#[cfg(target_os = "macos")]
#[inline]
fn read_i32(buf: &[u8], off: usize) -> i32 {
    i32::from_ne_bytes(buf[off..off + 4].try_into().unwrap())
}

#[cfg(target_os = "macos")]
#[inline]
fn read_i64(buf: &[u8], off: usize) -> i64 {
    i64::from_ne_bytes(buf[off..off + 8].try_into().unwrap())
}
