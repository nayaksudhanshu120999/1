use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Category {
    App,
    File,
    Calc,
    Web,
    Note,
}

impl Category {
    pub fn badge(self) -> &'static str {
        match self {
            Category::App => "APP",
            Category::File => "FILE",
            Category::Calc => "CALC",
            Category::Web => "WEB",
            Category::Note => "NOTE",
        }
    }
}

/// One indexable item. `title_lc` and `detail_lc` are pre-folded at build
/// time so scoring never allocates per keystroke.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Entry {
    pub title: String,
    pub detail: String,
    pub target: String,
    pub category: Category,
    #[serde(default)]
    pub title_lc: String,
    #[serde(default)]
    pub detail_lc: String,
}

impl Entry {
    pub fn new(title: impl Into<String>, detail: impl Into<String>, target: impl Into<String>, category: Category) -> Self {
        let title = title.into();
        let detail = detail.into();
        let title_lc = fold(&title);
        let detail_lc = fold(&detail);
        Self { title, detail, target, category, title_lc, detail_lc }
    }

    /// Cheap fixup after deserialization from an older cache.
    pub fn ensure_folded(&mut self) {
        if self.title_lc.is_empty() {
            self.title_lc = fold(&self.title);
        }
        if self.detail_lc.is_empty() {
            self.detail_lc = fold(&self.detail);
        }
    }
}

/// Lowercase, whitespace-collapsed, no allocation beyond the result string.
pub fn fold(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut last_space = false;
    for c in s.chars() {
        if c.is_whitespace() {
            if !last_space && !out.is_empty() {
                out.push(' ');
            }
            last_space = true;
        } else {
            last_space = false;
            for lc in c.to_lowercase() {
                out.push(lc);
            }
        }
    }
    out
}

#[derive(Debug, Clone)]
pub enum ExecutionOutcome {
    Launched,
    Copied(String),
    Failed(String),
}
