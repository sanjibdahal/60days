use std::path::PathBuf;
use std::sync::mpsc;
use std::thread;
use crate::fs::{Entry, list_dir, delete, rename, copy_entry, search};

#[derive(Debug, PartialEq)]
pub enum Mode {
    Normal,
    Rename,
    Search,
    Confirm(ConfirmAction),
}

#[derive(Debug, PartialEq)]
pub enum ConfirmAction {
    Delete,
    Copy,
}

pub struct App {
    pub current_dir:    PathBuf,
    pub entries:        Vec<Entry>,
    pub selected:       usize,
    pub mode:           Mode,
    pub input:          String,
    pub status:         String,
    pub clipboard:      Option<PathBuf>,
    pub search_results: Vec<Entry>,
    pub in_search:      bool,
    pub folder_size:    String,                  // displayed in preview
    size_rx:            mpsc::Receiver<String>,  // receives computed size from thread
    size_tx:            mpsc::Sender<String>,    // kept to clone for new threads
    last_selected:      Option<usize>,           // track selection change
}

impl App {
    pub fn new(start: PathBuf) -> Self {
        let entries          = list_dir(&start).unwrap_or_default();
        let (tx, rx)         = mpsc::channel();
        App {
            current_dir:    start,
            entries,
            selected:       0,
            mode:           Mode::Normal,
            input:          String::new(),
            status:         String::from("superfly | q:quit  d:delete  c:copy  p:paste  r:rename  /:search"),
            clipboard:      None,
            search_results: Vec::new(),
            in_search:      false,
            folder_size:    String::new(),
            size_rx:        rx,
            size_tx:        tx,
            last_selected:  None,
        }
    }

    /// call every frame from main loop — checks channel + triggers size calc on selection change
    pub fn tick(&mut self) {
        if let Ok(size) = self.size_rx.try_recv() {
            self.folder_size = size;
        }

        let current = Some(self.selected);
        if current != self.last_selected {
            self.last_selected = current;
            self.folder_size   = String::from("calculating...");
            self.spawn_size_calc();
        }
    }

    fn spawn_size_calc(&self) {
        if let Some(entry) = self.selected_entry() {
            if entry.is_dir {
                let path = entry.path.clone();
                let tx   = self.size_tx.clone();
                thread::spawn(move || {
                    let size = dir_size(&path);
                    let _ = tx.send(format_size(size));
                });
            } else {
                // for files just show the size directly
                let _ = self.size_tx.send(format_size(entry.size));
            }
        }
    }

    pub fn reload(&mut self) {
        self.entries       = list_dir(&self.current_dir).unwrap_or_default();
        self.selected      = self.selected.min(self.entries.len().saturating_sub(1));
        self.last_selected = None; // force size recalc
        self.folder_size   = String::new();
    }

    pub fn current_entries(&self) -> &Vec<Entry> {
        if self.in_search { &self.search_results } else { &self.entries }
    }

    pub fn selected_entry(&self) -> Option<&Entry> {
        self.current_entries().get(self.selected)
    }

    pub fn move_up(&mut self) {
        if self.selected > 0 { self.selected -= 1; }
    }

    pub fn move_down(&mut self) {
        let len = self.current_entries().len();
        if self.selected + 1 < len { self.selected += 1; }
    }

    pub fn enter(&mut self) {
        if let Some(entry) = self.selected_entry().cloned() {
            if entry.is_dir {
                self.current_dir = entry.path.clone();
                self.in_search   = false;
                self.selected    = 0;
                self.reload();
                self.status = format!("📁 {}", self.current_dir.display());
            }
        }
    }

    pub fn go_up(&mut self) {
        if let Some(parent) = self.current_dir.parent() {
            self.current_dir = parent.to_path_buf();
            self.in_search   = false;
            self.selected    = 0;
            self.reload();
            self.status = format!("📁 {}", self.current_dir.display());
        }
    }

    pub fn start_delete(&mut self) {
        if self.selected_entry().is_some() {
            let name    = self.selected_entry().unwrap().name.clone();
            self.mode   = Mode::Confirm(ConfirmAction::Delete);
            self.status = format!("Delete '{}'? (y/n)", name);
        }
    }

    pub fn confirm_delete(&mut self) {
        if let Some(entry) = self.selected_entry().cloned() {
            match delete(&entry.path) {
                Ok(_)  => { self.status = format!("Deleted '{}'", entry.name); self.reload(); }
                Err(e) => { self.status = format!("Error: {}", e); }
            }
        }
        self.mode = Mode::Normal;
    }

    pub fn copy(&mut self) {
        if let Some(entry) = self.selected_entry() {
            let path = entry.path.clone();
            let name = entry.name.clone();
            self.clipboard = Some(path);
            self.status    = format!("Copied '{}' to clipboard", name);
        }
    }

    pub fn paste(&mut self) {
        if let Some(src) = self.clipboard.clone() {
            let fname = src.file_name().unwrap_or_default().to_os_string();
            let dest  = self.current_dir.join(&fname);
            match copy_entry(&src, &dest) {
                Ok(_)  => { self.status = format!("Pasted '{}'", fname.to_string_lossy()); self.reload(); }
                Err(e) => { self.status = format!("Error: {}", e); }
            }
        } else {
            self.status = String::from("Nothing in clipboard");
        }
    }

    pub fn start_rename(&mut self) {
        if let Some(entry) = self.selected_entry() {
            self.input  = entry.name.clone();
            self.mode   = Mode::Rename;
            self.status = String::from("Rename: (Enter to confirm, Esc to cancel)");
        }
    }

    pub fn confirm_rename(&mut self) {
        if let Some(entry) = self.selected_entry().cloned() {
            let new_path = entry.path.parent().unwrap().join(&self.input);
            match rename(&entry.path, &new_path) {
                Ok(_)  => { self.status = format!("Renamed to '{}'", self.input); self.reload(); }
                Err(e) => { self.status = format!("Error: {}", e); }
            }
        }
        self.input = String::new();
        self.mode  = Mode::Normal;
    }

    pub fn start_search(&mut self) {
        self.input  = String::new();
        self.mode   = Mode::Search;
        self.status = String::from("Search: (Enter to confirm, Esc to cancel)");
    }

    pub fn confirm_search(&mut self) {
        if !self.input.is_empty() {
            self.search_results = search(&self.current_dir, &self.input);
            self.in_search      = true;
            self.selected       = 0;
            self.status         = format!("Found {} results for '{}'", self.search_results.len(), self.input);
        }
        self.mode = Mode::Normal;
    }

    pub fn cancel_input(&mut self) {
        self.input  = String::new();
        self.mode   = Mode::Normal;
        self.status = String::from("superfly | q:quit  d:delete  c:copy  p:paste  r:rename  /:search");
    }

    pub fn clear_search(&mut self) {
        self.in_search = false;
        self.selected  = 0;
        self.status    = String::from("Search cleared");
    }
}

fn dir_size(path: &PathBuf) -> u64 {
    let mut total = 0u64;
    if let Ok(entries) = std::fs::read_dir(path) {
        for entry in entries.filter_map(|e| e.ok()) {
            let p = entry.path();
            if p.is_dir() {
                total += dir_size(&p);
            } else {
                total += entry.metadata().map(|m| m.len()).unwrap_or(0);
            }
        }
    }
    total
}

fn format_size(size: u64) -> String {
    match size {
        s if s < 1024                => format!("{}  B", s),
        s if s < 1024 * 1024         => format!("{:.1} KB", s as f64 / 1024.0),
        s if s < 1024 * 1024 * 1024  => format!("{:.1} MB", s as f64 / (1024.0 * 1024.0)),
        s                            => format!("{:.2} GB", s as f64 / (1024.0 * 1024.0 * 1024.0)),
    }
}