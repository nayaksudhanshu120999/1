use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Bang {
    pub trigger: String,
    #[serde(default)]
    pub name: String,
    pub url: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub hotkey: String,
    pub max_results: usize,
    #[serde(default = "default_accent")]
    pub accent: String,
    pub scan_dirs: Vec<String>,
    #[serde(default)]
    pub bangs: Vec<Bang>,
}

fn default_accent() -> String {
    "#4c8dff".to_string()
}

impl Default for Config {
    fn default() -> Self {
        Self {
            hotkey: "ALT+SPACE".into(),
            max_results: 40,
            accent: default_accent(),
            scan_dirs: vec![
                r"%APPDATA%\Microsoft\Windows\Start Menu\Programs".into(),
                r"%PROGRAMDATA%\Microsoft\Windows\Start Menu\Programs".into(),
                r"%USERPROFILE%\Desktop".into(),
                r"%PUBLIC%\Desktop".into(),
            ],
            bangs: vec![
                Bang { trigger: "g".into(), name: "Google".into(), url: "https://www.google.com/search?q={}".into() },
                Bang { trigger: "gh".into(), name: "GitHub".into(), url: "https://github.com/search?q={}".into() },
                Bang { trigger: "w".into(), name: "Wikipedia".into(), url: "https://en.wikipedia.org/w/index.php?search={}".into() },
            ],
        }
    }
}

pub fn data_dir() -> PathBuf {
    let base = std::env::var("APPDATA")
        .or_else(|_| std::env::var("HOME"))
        .unwrap_or_else(|_| ".".to_string());
    let dir = PathBuf::from(base).join("cmdpal");
    let _ = std::fs::create_dir_all(&dir);
    dir
}

impl Config {
    pub fn path() -> PathBuf {
        data_dir().join("config.toml")
    }

    pub fn cache_path() -> PathBuf {
        data_dir().join("index-cache.json")
    }

    pub fn notes_path() -> PathBuf {
        data_dir().join("notes.txt")
    }

    /// Load config, writing a commented default file on first run.
    pub fn load() -> Self {
        let path = Self::path();
        if !path.exists() {
            let cfg = Config::default();
            let _ = std::fs::write(&path, toml::to_string_pretty(&cfg).unwrap_or_default());
            return cfg;
        }
        match std::fs::read_to_string(&path) {
            Ok(text) => toml::from_str(&text).unwrap_or_default(),
            Err(_) => Config::default(),
        }
    }

    /// Expand `%VAR%` segments in a configured path.
    pub fn expand_dirs(&self) -> Vec<PathBuf> {
        self.scan_dirs
            .iter()
            .map(|raw| PathBuf::from(expand_env(raw)))
            .filter(|p| p.exists())
            .collect()
    }
}

pub fn expand_env(raw: &str) -> String {
    let mut out = String::with_capacity(raw.len());
    let mut rest = raw;
    while let Some(start) = rest.find('%') {
        out.push_str(&rest[..start]);
        let after = &rest[start + 1..];
        match after.find('%') {
            Some(end) => {
                match std::env::var(&after[..end]) {
                    Ok(val) => out.push_str(&val),
                    Err(_) => out.push_str(&after[..end]),
                }
                rest = &after[end + 1..];
            }
            None => {
                out.push_str(&rest[start..]);
                return out;
            }
        }
    }
    out.push_str(rest);
    out
}
