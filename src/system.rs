use std::fs;
use std::path::PathBuf;
use std::time::Duration;

use crate::config::data_dir;

/// Single-instance guard via a lock file. Cheaper and simpler than a named
/// Win32 mutex, and it survives crashes because a stale lock expires.
pub struct Instance(PathBuf);

impl Drop for Instance {
    fn drop(&mut self) {
        let _ = fs::remove_file(&self.0);
    }
}

pub fn claim_instance() -> Option<Instance> {
    let path = data_dir().join("cmdpal.lock");
    if let Ok(meta) = fs::metadata(&path) {
        if let Ok(age) = meta.modified().and_then(|m| m.elapsed().or(Ok(Duration::ZERO))) {
            if age < Duration::from_secs(60 * 60 * 6) {
                return None;
            }
        }
    }
    fs::write(&path, std::process::id().to_string()).ok()?;
    Some(Instance(path))
}

#[cfg(windows)]
mod hotkey {
    use windows::Win32::UI::Input::KeyboardAndMouse::{
        RegisterHotKey, UnregisterHotKey, HOT_KEY_MODIFIERS, MOD_ALT, MOD_CONTROL, MOD_SHIFT,
        MOD_WIN, VK_SPACE,
    };
    use windows::Win32::UI::WindowsAndMessaging::{GetMessageW, MSG, WM_HOTKEY};

    const HOTKEY_ID: i32 = 1;

    pub fn parse_spec(spec: &str) -> (HOT_KEY_MODIFIERS, u32) {
        let mut mods = HOT_KEY_MODIFIERS(0);
        let mut vk = VK_SPACE.0;
        for part in spec.split('+') {
            match part.trim().to_ascii_uppercase().as_str() {
                "ALT" => mods |= MOD_ALT,
                "CTRL" | "CONTROL" => mods |= MOD_CONTROL,
                "SHIFT" => mods |= MOD_SHIFT,
                "WIN" | "SUPER" => mods |= MOD_WIN,
                "SPACE" => vk = VK_SPACE.0,
                other => {
                    if let Some(c) = other.chars().next() {
                        // Virtual-key codes for A-Z and 0-9 match ASCII.
                        if c.is_ascii_alphanumeric() {
                            vk = c.to_ascii_uppercase() as u32;
                        }
                    }
                }
            }
        }
        (mods, vk)
    }

    pub fn spawn(spec: String, on_press: impl Fn() + Send + 'static) -> bool {
        let (mods, vk) = parse_spec(&spec);
        std::thread::spawn(move || unsafe {
            if RegisterHotKey(None, HOTKEY_ID, mods, vk).is_err() {
                eprintln!("cmdpal: hotkey {spec} unavailable (already taken?)");
                return;
            }
            let mut msg = MSG::default();
            while GetMessageW(&mut msg, None, 0, 0).as_bool() {
                if msg.message == WM_HOTKEY {
                    on_press();
                }
            }
            let _ = UnregisterHotKey(None, HOTKEY_ID);
        });
        true
    }
}

#[cfg(not(windows))]
mod hotkey {
    pub fn spawn(_spec: String, _on_press: impl Fn() + Send + 'static) -> bool {
        eprintln!("cmdpal: global hotkey is Windows-only; run the binary to use the UI");
        false
    }
}

pub fn spawn_hotkey(spec: &str, on_press: impl Fn() + Send + 'static) -> bool {
    hotkey::spawn(spec.to_string(), on_press)
}
