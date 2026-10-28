use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use crate::config::{expand_env, Config};
use crate::models::{Category, Entry};

/// Cache is considered fresh for this long; after that a rescan runs anyway.
const FRESH_SECS: u64 = 15 * 60;
const SKIP_DIRS: &[&str] = &["WindowsApps", "node_modules", ".git", "$RECYCLE.BIN", "System Volume Information"];

/// Walk the configured Start Menu / Desktop folders and build entries.
/// Runs on a worker thread; never touches the UI.
pub fn scan(cfg: &Config) -> Vec<Entry> {
    let mut out: Vec<Entry> = Vec::with_capacity(512);
    let mut seen: std::collections::HashSet<String> = std::collections::HashSet::new();

    for dir in cfg.expand_dirs() {
        walk(&dir, 0, &mut out, &mut seen);
    }

    out.sort_by(|a, b| a.title_lc.cmp(&b.title_lc));
    out
}

fn walk(dir: &Path, depth: usize, out: &mut Vec<Entry>, seen: &mut std::collections::HashSet<String>) {
    if depth > 4 {
        return;
    }
    let read = match fs::read_dir(dir) {
        Ok(r) => r,
        Err(_) => return,
    };

    for item in read.flatten() {
        let path = item.path();
        let name = item.file_name();
        let name = name.to_string_lossy();

        let is_dir = item.file_type().map(|t| t.is_dir()).unwrap_or(false);
        if is_dir {
            if SKIP_DIRS.iter().any(|s| name.eq_ignore_ascii_case(s)) || name.starts_with('.') {
                continue;
            }
            walk(&path, depth + 1, out, seen);
            continue;
        }

        let ext = path
            .extension()
            .map(|e| e.to_string_lossy().to_ascii_lowercase())
            .unwrap_or_default();
        let category = match ext.as_str() {
            "lnk" => Category::App,
            "exe" => Category::App,
            "url" => Category::Web,
            "bat" | "cmd" | "ps1" => Category::App,
            _ => continue,
        };

        let title = humanize(&path, &name, &ext);
        let key = title.to_lowercase();
        if !seen.insert(key) {
            continue;
        }

        let folder = path
            .parent()
            .and_then(|p| p.file_name())
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_default();

        out.push(Entry::new(
            title,
            if folder.is_empty() { path.display().to_string() } else { folder },
            path.display().to_string(),
            category,
        ));
    }
}

fn humanize(path: &Path, file_name: &str, ext: &str) -> String {
    let stem = file_name
        .strip_suffix(&format!(".{ext}"))
        .unwrap_or(file_name)
        .to_string();
    // "Visual Studio Code.lnk" -> "Visual Studio Code"; "code" stays "code"
    if stem.is_empty() {
        path.display().to_string()
    } else {
        stem
    }
}

pub fn load_cache() -> Option<Vec<Entry>> {
    let path = Config::cache_path();
    let meta = fs::metadata(&path).ok()?;
    let age = SystemTime::now()
        .duration_since(meta.modified().ok()?)
        .ok()?
        .as_secs();
    if age > FRESH_SECS {
        return None;
    }
    let text = fs::read_to_string(&path).ok()?;
    let mut entries: Vec<Entry> = serde_json::from_str(&text).ok()?;
    if entries.is_empty() {
        return None;
    }
    for e in entries.iter_mut() {
        e.ensure_folded();
    }
    Some(entries)
}

pub fn save_cache(entries: &[Entry]) {
    let path = Config::cache_path();
    let tmp = path.with_extension("json.tmp");
    if let Ok(json) = serde_json::to_string(entries) {
        if fs::write(&tmp, json).is_ok() {
            let _ = fs::rename(&tmp, &path);
        }
    }
}

/// Where config points, resolved, for display in the UI status line.
pub fn summary(cfg: &Config) -> String {
    let dirs: Vec<PathBuf> = cfg.expand_dirs();
    format!("{} folders configured", dirs.len())
}

pub fn expand(p: &str) -> String {
    expand_env(p)
}
