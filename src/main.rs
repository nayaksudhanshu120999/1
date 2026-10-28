mod config;
mod engine;
mod executor;
mod indexer;
mod models;
mod system;

use std::cell::RefCell;
use std::rc::Rc;
use std::sync::{Arc, Mutex};
use std::thread;

use engine::Engine;
use models::{ExecutionOutcome, Entry};

slint::include_modules!();

thread_local! {
    static ROWS: RefCell<Option<Rc<slint::VecModel<Row>>>> = const { RefCell::new(None) };
    static CURRENT: RefCell<Vec<Entry>> = const { RefCell::new(Vec::new()) };
}

fn main() -> Result<(), slint::PlatformError> {
    let _instance = match system::claim_instance() {
        Some(g) => g,
        None => {
            eprintln!("cmdpal: already running");
            std::process::exit(0);
        }
    };

    let cfg = config::Config::load();
    let accent = cfg
        .accent
        .parse::<slint::Color>()
        .unwrap_or(slint::Color::from_rgb_u8(0x4c, 0x8d, 0xff));

    let engine = Arc::new(Mutex::new(Engine::new(&cfg)));

    // 1. Paint from cache immediately - cold start is a file read, not a scan.
    if let Some(cached) = indexer::load_cache() {
        let mut eng = engine.lock().unwrap();
        eng.set_entries(cached);
    }

    let ui = PaletteWindow::new()?;
    ui.set_accent(accent);
    ui.set_status(slint::SharedString::from("Loading index..."));

    let model = Rc::new(slint::VecModel::<Row>::new());
    ui.set_rows(model.clone().into());
    ROWS.with(|r| *r.borrow_mut() = Some(model.clone()));

    refresh(&ui, &engine);

    // 2. Re-scan on a worker thread and swap in the results when done.
    {
        let engine = engine.clone();
        let weak = ui.as_weak();
        let cfg = cfg.clone();
        thread::spawn(move || {
            let fresh = indexer::scan(&cfg);
            let count = fresh.len();
            indexer::save_cache(&fresh);
            let engine_on_ui = engine.clone();
            let _ = slint::invoke_from_event_loop(move || {
                engine_on_ui.lock().unwrap().set_entries(fresh);
                if let Some(ui) = weak.upgrade() {
                    ui.set_status(slint::SharedString::from(format!("{count} items indexed")));
                    refresh(&ui, &engine_on_ui);
                }
            });
        });
    }

    // 3. Input wiring. Everything below runs on the UI thread.
    {
        let engine = engine.clone();
        let weak = ui.as_weak();
        ui.on_query_changed(move |_| {
            if let Some(ui) = weak.upgrade() {
                refresh(&ui, &engine);
            }
        });
    }

    {
        let weak = ui.as_weak();
        ui.on_move_down(move || {
            if let Some(ui) = weak.upgrade() {
                let n = CURRENT.with(|c| c.borrow().len()) as i32;
                if n > 0 {
                    ui.set_selected((ui.get_selected() + 1).min(n - 1));
                }
            }
        });
    }

    {
        let weak = ui.as_weak();
        ui.on_move_up(move || {
            if let Some(ui) = weak.upgrade() {
                ui.set_selected((ui.get_selected() - 1).max(0));
            }
        });
    }

    {
        let weak = ui.as_weak();
        ui.on_dismissed(move || {
            if let Some(ui) = weak.upgrade() {
                ui.window().hide();
            }
        });
    }

    {
        let weak = ui.as_weak();
        ui.on_accepted(move || {
            let Some(ui) = weak.upgrade() else { return };
            let idx = ui.get_selected().max(0) as usize;
            let entry = CURRENT.with(|c| c.borrow().get(idx).cloned());
            let Some(entry) = entry else { return };
            match executor::execute(&entry) {
                ExecutionOutcome::Launched | ExecutionOutcome::Copied(_) => ui.window().hide(),
                ExecutionOutcome::Failed(err) => {
                    ui.set_status(slint::SharedString::from(format!("Failed: {err}")))
                }
            }
        });
    }

    // 4. Global hotkey toggles the window.
    {
        let weak = ui.as_weak();
        system::spawn_hotkey(&cfg.hotkey, move || {
            let weak = weak.clone();
            let _ = slint::invoke_from_event_loop(move || {
                if let Some(ui) = weak.upgrade() {
                    if ui.window().is_visible() {
                        ui.window().hide();
                    } else {
                        ui.show();
                        ui.set_query(slint::SharedString::default());
                        ui.invoke_focus_input();
                    }
                }
            });
        });
    }

    ui.invoke_focus_input();
    ui.run()
}

/// Recompute results for the current query and push only the visible rows.
fn refresh(ui: &PaletteWindow, engine: &Arc<Mutex<Engine>>) {
    let query = ui.get_query().to_string();
    let hits = engine.lock().unwrap().search(&query);

    let entries: Vec<Entry> = hits.iter().map(|h| h.entry.clone()).collect();
    let rows: Vec<Row> = hits
        .iter()
        .map(|h| Row {
            title: h.entry.title.clone().into(),
            detail: h.entry.detail.clone().into(),
            badge: h.entry.category.badge().into(),
        })
        .collect();
    let total = rows.len();

    ROWS.with(|r| {
        if let Some(model) = r.borrow().as_ref() {
            model.set_row_count(rows.len(), empty_row());
            for (i, row) in rows.into_iter().enumerate() {
                model.set_row_data(i, row);
            }
        }
    });
    CURRENT.with(|c| *c.borrow_mut() = entries);
    ui.set_selected(0);
    if query.trim().is_empty() {
        ui.set_status(slint::SharedString::from(format!("{total} items. Enter to run, Esc to hide.")));
    }
}

fn empty_row() -> Row {
    Row {
        title: slint::SharedString::default(),
        detail: slint::SharedString::default(),
        badge: slint::SharedString::default(),
    }
}
