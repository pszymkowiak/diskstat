use std::fs;
use std::io::Write;
use std::path::Path;

use crate::types::FileTree;

/// Delete a file or directory (moves to trash on macOS when possible).
pub fn delete_path(path: &Path) -> Result<(), String> {
    if !path.exists() {
        return Err(format!("Path does not exist: {}", path.display()));
    }

    // Try to move to trash using macOS command
    let result = std::process::Command::new("osascript")
        .args([
            "-e",
            &format!(
                "tell application \"Finder\" to delete POSIX file \"{}\"",
                path.display()
            ),
        ])
        .output();

    match result {
        Ok(output) if output.status.success() => Ok(()),
        _ => {
            // Fallback: direct delete
            if path.is_dir() {
                fs::remove_dir_all(path)
                    .map_err(|e| format!("Failed to delete directory: {}", e))
            } else {
                fs::remove_file(path)
                    .map_err(|e| format!("Failed to delete file: {}", e))
            }
        }
    }
}

/// Open a path in Finder.
pub fn open_in_finder(path: &Path) -> Result<(), String> {
    open::that(path).map_err(|e| format!("Failed to open: {}", e))
}

/// Export tree data to CSV.
pub fn export_csv(tree: &FileTree) -> Result<String, String> {
    let timestamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    let filename = format!("diskstat-{}.csv", timestamp);

    let file = fs::File::create(&filename)
        .map_err(|e| format!("Failed to create {}: {}", filename, e))?;
    let mut w = std::io::BufWriter::new(file);

    writeln!(w, "path,size,is_dir,extension")
        .map_err(|e| format!("Write error: {}", e))?;

    for nid in tree.root.descendants(&tree.arena) {
        let path = tree.full_path(nid);
        let entry = tree.arena[nid].get();
        let path_str = escape_csv(&path.to_string_lossy());
        let ext = entry.extension.as_deref().unwrap_or("");
        writeln!(w, "{},{},{},{}", path_str, entry.size, entry.is_dir, escape_csv(ext))
            .map_err(|e| format!("Write error: {}", e))?;
    }

    w.flush().map_err(|e| format!("Flush error: {}", e))?;
    Ok(filename)
}

/// Escape a string for CSV (quote if contains comma, quote, or newline).
fn escape_csv(s: &str) -> String {
    if s.contains(',') || s.contains('"') || s.contains('\n') {
        format!("\"{}\"", s.replace('"', "\"\""))
    } else {
        s.to_string()
    }
}

/// Copy path to clipboard.
pub fn copy_to_clipboard(path: &Path) -> Result<(), String> {
    use arboard::Clipboard;
    let mut clipboard =
        Clipboard::new().map_err(|e| format!("Failed to access clipboard: {}", e))?;
    clipboard
        .set_text(path.to_string_lossy().to_string())
        .map_err(|e| format!("Failed to copy: {}", e))
}
