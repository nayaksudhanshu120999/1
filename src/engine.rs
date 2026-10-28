use crate::config::{Bang, Config};
use crate::models::{fold, Category, Entry};

/// A scored search result. `tier` is the primary sort key (lower is better),
/// `score` breaks ties inside a tier.
#[derive(Debug, Clone)]
pub struct Hit {
    pub entry: Entry,
    pub tier: u8,
    pub score: i32,
}

pub struct Engine {
    entries: Vec<Entry>,
    bangs: Vec<Bang>,
    max_results: usize,
    epoch: u64,
}

impl Engine {
    pub fn new(cfg: &Config) -> Self {
        Self {
            entries: Vec::new(),
            bangs: cfg.bangs.clone(),
            max_results: cfg.max_results.max(1),
            epoch: 0,
        }
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn epoch(&self) -> u64 {
        self.epoch
    }

    /// Swap in a freshly scanned index. Called from the UI thread only.
    pub fn set_entries(&mut self, mut fresh: Vec<Entry>) {
        for e in fresh.iter_mut() {
            e.ensure_folded();
        }
        self.entries = fresh;
        self.epoch += 1;
    }

    pub fn search(&self, raw: &str) -> Vec<Hit> {
        let query = raw.trim();
        if query.is_empty() {
            return self.idle_list();
        }

        // 1. calculator: "=expr" or an expression containing an operator
        if let Some(hit) = self.try_calc(query) {
            return vec![hit];
        }

        // 2. bangs: "g rust async"
        if let Some(hit) = self.try_bang(query) {
            return vec![hit];
        }

        // 3. index search
        let needle = fold(query);
        let mut hits: Vec<Hit> = Vec::with_capacity(64);
        for entry in &self.entries {
            if let Some((tier, score)) = score_entry(entry, &needle) {
                hits.push(Hit { entry: entry.clone(), tier, score });
            }
        }
        // Stable, allocation-free ordering: tier, then score, then length.
        hits.sort_unstable_by(|a, b| {
            a.tier
                .cmp(&b.tier)
                .then_with(|| b.score.cmp(&a.score))
                .then_with(|| a.entry.title.len().cmp(&b.entry.title.len()))
        });
        hits.truncate(self.max_results);
        hits
    }

    fn idle_list(&self) -> Vec<Hit> {
        self.entries
            .iter()
            .take(self.max_results)
            .map(|e| Hit { entry: e.clone(), tier: 9, score: 0 })
            .collect()
    }

    fn try_calc(&self, query: &str) -> Option<Hit> {
        let expr = query.strip_prefix('=').map(str::trim).unwrap_or(query);
        if !expr.chars().any(|c| "+-*/%^()".contains(c)) || !expr.chars().any(|c| c.is_ascii_digit()) {
            return None;
        }
        let value = eval(expr)?;
        let pretty = format_num(value);
        Some(Hit {
            entry: Entry::new(
                pretty.clone(),
                format!("{expr}   -  Enter to copy"),
                pretty,
                Category::Calc,
            ),
            tier: 0,
            score: 1000,
        })
    }

    fn try_bang(&self, query: &str) -> Option<Hit> {
        let (trigger, rest) = query.split_once(' ')?;
        let bang = self.bangs.iter().find(|b| b.trigger.eq_ignore_ascii_case(trigger))?;
        if rest.trim().is_empty() {
            return None;
        }
        let url = bang.url.replace("{}", &encode(rest.trim()));
        let name = if bang.name.is_empty() { trigger } else { &bang.name };
        Some(Hit {
            entry: Entry::new(
                format!("{name}: {}", rest.trim()),
                url.clone(),
                url,
                Category::Web,
            ),
            tier: 0,
            score: 900,
        })
    }
}

/// 7-tier ranking. Returns `(tier, score)`; `None` means "does not match".
/// No allocation happens in here - it walks pre-folded strings.
fn score_entry(e: &Entry, needle: &str) -> Option<(u8, i32)> {
    let title = &e.title_lc;

    if title == needle {
        return Some((0, 100));
    }
    if let Some(rest) = title.strip_prefix(needle) {
        return Some((1, 90 - rest.chars().count().min(30) as i32));
    }
    if initials_match(title, needle) {
        return Some((2, 80));
    }
    if let Some(pos) = title.find(needle) {
        let at_word_start = pos == 0 || title.as_bytes()[pos - 1] == b' ';
        return Some((if at_word_start { 2 } else { 3 }, 70 - pos.min(40) as i32));
    }
    if let Some(s) = subsequence(needle, title) {
        return Some((4, s));
    }
    if e.detail_lc.contains(needle) {
        return Some((5, 20));
    }
    if let Some(s) = subsequence(needle, &e.detail_lc) {
        return Some((6, s / 2));
    }
    None
}

/// "vs code" matches "visual studio code" via v-s-c.
fn initials_match(title: &str, needle: &str) -> bool {
    let mut initials = String::with_capacity(8);
    let mut word_start = true;
    for c in title.chars() {
        if c == ' ' || c == '-' || c == '_' || c == '.' {
            word_start = true;
        } else if word_start {
            initials.push(c);
            word_start = false;
            if initials.len() > 12 {
                break;
            }
        }
    }
    initials.starts_with(needle)
}

/// Subsequence match with a bonus for adjacency and word boundaries.
fn subsequence(needle: &str, hay: &str) -> Option<i32> {
    let mut score = 0i32;
    let mut hay_chars = hay.char_indices().peekable();
    let mut prev_end: Option<usize> = None;

    for n in needle.chars() {
        let mut found = None;
        while let Some(&(i, h)) = hay_chars.peek() {
            if h == n {
                found = Some(i);
                hay_chars.next();
                break;
            }
            hay_chars.next();
        }
        let i = found?;
        match prev_end {
            Some(p) if i == p + 1 => score += 6,
            Some(p) => score -= ((i - p) as i32).min(10),
            None => score -= (i as i32).min(10),
        }
        if i == 0 || hay.as_bytes().get(i.wrapping_sub(1)) == Some(&b' ') {
            score += 4;
        }
        prev_end = Some(i);
    }
    Some(score)
}

// ---------------------------------------------------------------------------
// Minimal expression evaluator: + - * / % ^ and parentheses. No unsafe, no
// dependencies, no regex. Rejects anything it does not fully consume.
// ---------------------------------------------------------------------------
pub fn eval(input: &str) -> Option<f64> {
    let mut p = Parser { s: input.as_bytes(), i: 0 };
    let v = p.expr()?;
    p.ws();
    if p.i != p.s.len() {
        return None;
    }
    if v.is_finite() {
        Some(v)
    } else {
        None
    }
}

struct Parser<'a> {
    s: &'a [u8],
    i: usize,
}

impl<'a> Parser<'a> {
    fn ws(&mut self) {
        while self.i < self.s.len() && self.s[self.i].is_ascii_whitespace() {
            self.i += 1;
        }
    }

    fn peek(&mut self) -> Option<u8> {
        self.ws();
        self.s.get(self.i).copied()
    }

    fn expr(&mut self) -> Option<f64> {
        let mut v = self.term()?;
        loop {
            match self.peek() {
                Some(b'+') => {
                    self.i += 1;
                    v += self.term()?;
                }
                Some(b'-') => {
                    self.i += 1;
                    v -= self.term()?;
                }
                _ => return Some(v),
            }
        }
    }

    fn term(&mut self) -> Option<f64> {
        let mut v = self.power()?;
        loop {
            match self.peek() {
                Some(b'*') => {
                    self.i += 1;
                    v *= self.power()?;
                }
                Some(b'/') => {
                    self.i += 1;
                    let d = self.power()?;
                    if d == 0.0 {
                        return None;
                    }
                    v /= d;
                }
                Some(b'%') => {
                    self.i += 1;
                    let d = self.power()?;
                    if d == 0.0 {
                        return None;
                    }
                    v %= d;
                }
                _ => return Some(v),
            }
        }
    }

    fn power(&mut self) -> Option<f64> {
        let base = self.unary()?;
        if self.peek() == Some(b'^') {
            self.i += 1;
            let exp = self.power()?;
            return Some(base.powf(exp));
        }
        Some(base)
    }

    fn unary(&mut self) -> Option<f64> {
        match self.peek() {
            Some(b'-') => {
                self.i += 1;
                Some(-self.unary()?)
            }
            Some(b'+') => {
                self.i += 1;
                self.unary()
            }
            _ => self.primary(),
        }
    }

    fn primary(&mut self) -> Option<f64> {
        match self.peek()? {
            b'(' => {
                self.i += 1;
                let v = self.expr()?;
                if self.peek()? != b')' {
                    return None;
                }
                self.i += 1;
                Some(v)
            }
            c if c.is_ascii_digit() || c == b'.' => {
                let start = self.i;
                while self.i < self.s.len()
                    && (self.s[self.i].is_ascii_digit() || self.s[self.i] == b'.')
                {
                    self.i += 1;
                }
                std::str::from_utf8(&self.s[start..self.i]).ok()?.parse().ok()
            }
            _ => None,
        }
    }
}

fn format_num(v: f64) -> String {
    if (v - v.round()).abs() < 1e-9 && v.abs() < 1e15 {
        format!("{}", v.round() as i64)
    } else {
        let s = format!("{v:.10}");
        s.trim_end_matches('0').trim_end_matches('.').to_string()
    }
}

/// Percent-encode for query strings without pulling in a url crate.
fn encode(s: &str) -> String {
    let mut out = String::with_capacity(s.len() * 3);
    for b in s.bytes() {
        match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => out.push(b as char),
            b' ' => out.push('+'),
            _ => out.push_str(&format!("%{b:02X}")),
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::Category;

    fn e(t: &str) -> Entry {
        Entry::new(t, "", t, Category::App)
    }

    #[test]
    fn tiers_order_correctly() {
        let eng = {
            let mut c = Config::default();
            c.bangs.clear();
            let mut en = Engine::new(&c);
            en.set_entries(vec![e("Visual Studio Code"), e("Code::Blocks"), e("Xcode")]);
            en
        };
        let hits = eng.search("code");
        assert_eq!(hits[0].entry.title, "Code::Blocks");
    }

    #[test]
    fn arithmetic() {
        assert_eq!(eval("2+3*4"), Some(14.0));
        assert_eq!(eval("(2+3)*4"), Some(20.0));
        assert_eq!(eval("2^10"), Some(1024.0));
        assert_eq!(eval("2+"), None);
    }

    #[test]
    fn bang_expands() {
        let mut c = Config::default();
        c.bangs = vec![Bang { trigger: "g".into(), name: "Google".into(), url: "https://g/?q={}".into() }];
        let eng = Engine::new(&c);
        let hits = eng.search("g rust async");
        assert_eq!(hits[0].entry.target, "https://g/?q=rust+async");
    }
}
