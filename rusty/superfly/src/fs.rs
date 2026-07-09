use std::fs;
use std::path::{Path, PathBuf};
use std::io;

#[derive(Debug, Clone)]
pub struct Entry {
    pub name: String,
    pub path: PathBuf,
    pub is_dir: bool,
    pub size: u64,
}

impl Entry {
    pub fn size_display(&self) -> String {
        if self.is_dir {
            return String::from("DIR");
        }
        match self.size {
            s if s < 1024 => format!("{}B", s),
            s if s < 1024 * 1024 => format!("{:.1}K", s as f64 / 1024.0),
            s if s < 1024 * 1024 * 1024 => format!("{:.1}M", s as f64 / (1024.0 * 1024.0)),
            s => format!("{:.1}G", s as f64 / (1024.0 * 1024.0 * 1024.0)),
        }
    }
}

pub fn list_dir(path: &Path) -> io::Result<Vec<Entry>> {
    let mut entries: Vec<Entry> = fs::read_dir(path)?
        .filter_map(|e| e.ok())
        .map(|e| {
            let meta    = e.metadata().ok();
            let is_dir  = meta.as_ref().map(|m| m.is_dir()).unwrap_or(false);
            let size    = meta.as_ref().map(|m| m.len()).unwrap_or(0);
            Entry {
                name: e.file_name().to_string_lossy().to_string(),
                path: e.path(),
                is_dir,
                size,
            }
        })
        .collect();

    // dirs first, then files, both sorted alphabetically
    entries.sort_by(|a, b| {
        b.is_dir.cmp(&a.is_dir).then(a.name.to_lowercase().cmp(&b.name.to_lowercase()))
    });

    Ok(entries)
}

pub fn delete(path: &Path) -> io::Result<()> {
    if path.is_dir() {
        fs::remove_dir_all(path)
    } else {
        fs::remove_file(path)
    }
}

pub fn rename(from: &Path, to: &Path) -> io::Result<()> {
    fs::rename(from, to)
}

pub fn copy_entry(from: &Path, to: &Path) -> io::Result<()> {
    if from.is_dir() {
        copy_dir_all(from, to)
    } else {
        // auto-rename if destination exists
        let dest = if to.exists() {
            let stem = to.file_stem().unwrap_or_default().to_string_lossy();
            let ext  = to.extension().map(|e| format!(".{}", e.to_string_lossy())).unwrap_or_default();
            to.parent().unwrap().join(format!("{}_copy{}", stem, ext))
        } else {
            to.to_path_buf()
        };
        fs::copy(from, dest)?;
        Ok(())
    }
}

fn copy_dir_all(src: &Path, dst: &Path) -> io::Result<()> {
    fs::create_dir_all(dst)?;
    for entry in fs::read_dir(src)? {
        let entry = entry?;
        let ty    = entry.file_type()?;
        if ty.is_dir() {
            copy_dir_all(&entry.path(), &dst.join(entry.file_name()))?;
        } else {
            fs::copy(entry.path(), dst.join(entry.file_name()))?;
        }
    }
    Ok(())
}

pub fn preview(path: &Path) -> String {
    if path.is_dir() {
        match list_dir(path) {
            Ok(entries) => entries
                .iter()
                .take(30)
                .map(|e| if e.is_dir { format!("📁 {}", e.name) } else { format!("  {}", e.name) })
                .collect::<Vec<_>>()
                .join("\n"),
            Err(e) => format!("Error: {}", e),
        }
    } else {
        match fs::read_to_string(path) {
            Ok(content) => content.lines().take(100).collect::<Vec<_>>().join("\n"),
            Err(_)      => String::from("[binary file]"),
        }
    }
}

pub fn search(root: &Path, query: &str) -> Vec<Entry> {
    let mut results = Vec::new();
    search_recursive(root, query, &mut results, 0);
    results
}

fn search_recursive(path: &Path, query: &str, results: &mut Vec<Entry>, depth: usize) {
    if depth > 5 { return; }
    if let Ok(entries) = list_dir(path) {
        for entry in entries {
            if entry.name.to_lowercase().contains(&query.to_lowercase()) {
                results.push(entry.clone());
            }
            if entry.is_dir {
                search_recursive(&entry.path, query, results, depth + 1);
            }
        }
    }
}
