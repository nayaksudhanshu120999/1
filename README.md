# cmdpal

A small, fast command palette for Windows, written in Rust with a Slint UI.
Type to search installed apps and shortcuts, do arithmetic inline, and fire
web searches with bangs.

## Build

```bash
cargo build --release
# target\release\cmdpal.exe
```

Requires a recent stable Rust and the MSVC toolchain on Windows.

## Use

- `Alt+Space` toggles the palette (configurable).
- Type to filter. `Up`/`Down` to move, `Enter` to run, `Esc` to hide.
- `12*8+3` or `=sqrt` style arithmetic is evaluated inline; `Enter` copies it.
- `g rust async` sends the query to Google. Bangs live in `config.toml`.
- Config and cache live in `%APPDATA%\cmdpal\`.

## Design notes

Everything is built around three rules:

1. **The window paints before the index exists.** Cached entries are loaded
   from disk, shown immediately, then a background thread rescans.
2. **Nothing per-keystroke allocates in proportion to corpus size.** Entries
   carry pre-folded lowercase strings; scoring walks chars, no `to_lowercase`.
3. **Only visible rows cross the UI boundary.** At most `max_results` rows are
   pushed into the Slint model, so layout cost is constant.
