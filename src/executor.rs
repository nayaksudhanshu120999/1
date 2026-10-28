use std::io::Write;
use std::process::{Command, Stdio};

use crate::models::{Category, Entry, ExecutionOutcome};

#[cfg(windows)]
use std::os::windows::process::CommandExt;
#[cfg(windows)]
const CREATE_NO_WINDOW: u32 = 0x0800_0000;

fn quiet(cmd: &mut Command) -> &mut Command {
    #[cfg(windows)]
    {
        cmd.creation_flags(CREATE_NO_WINDOW);
    }
    cmd
}

/// Run the entry. Never blocks longer than a process spawn, and never
/// inherits the parent's stdio (which would keep console handles alive).
pub fn execute(entry: &Entry) -> ExecutionOutcome {
    match entry.category {
        Category::Calc => copy_to_clipboard(&entry.target),
        Category::Web => open_url(&entry.target),
        Category::Note => append_note(&entry.target),
        Category::App | Category::File => launch_path(&entry.target),
    }
}

pub fn open_url(url: &str) -> ExecutionOutcome {
    // `start` handles both http(s) and registered protocol handlers.
    let mut cmd = Command::new("cmd");
    cmd.args(["/C", "start", "", url]);
    match quiet(&mut cmd).spawn() {
        Ok(_) => ExecutionOutcome::Launched,
        Err(e) => ExecutionOutcome::Failed(e.to_string()),
    }
}

pub fn launch_path(path: &str) -> ExecutionOutcome {
    match quiet(Command::new(path)).spawn() {
        Ok(_) => ExecutionOutcome::Launched,
        // .lnk and shell items need the shell to resolve them
        Err(_) => match quiet(Command::new("cmd")).args(["/C", "start", "", path]).spawn() {
            Ok(_) => ExecutionOutcome::Launched,
            Err(e) => ExecutionOutcome::Failed(e.to_string()),
        },
    }
}

pub fn launch_elevated(path: &str) -> ExecutionOutcome {
    let script = format!(
        "Start-Process -FilePath '{}' -Verb RunAs",
        path.replace('\'', "''")
    );
    match quiet(Command::new("powershell"))
        .args(["-NoProfile", "-WindowStyle", "Hidden", "-Command", &script])
        .spawn()
    {
        Ok(_) => ExecutionOutcome::Launched,
        Err(e) => ExecutionOutcome::Failed(e.to_string()),
    }
}

pub fn copy_to_clipboard(text: &str) -> ExecutionOutcome {
    let child = quiet(Command::new("cmd"))
        .args(["/C", "clip"])
        .stdin(Stdio::piped())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn();
    match child {
        Ok(mut c) => {
            if let Some(stdin) = c.stdin.as_mut() {
                let _ = stdin.write_all(text.as_bytes());
            }
            let _ = c.wait();
            ExecutionOutcome::Copied(text.to_string())
        }
        Err(e) => ExecutionOutcome::Failed(e.to_string()),
    }
}

fn append_note(text: &str) -> ExecutionOutcome {
    use std::fs::OpenOptions;
    let path = crate::config::Config::notes_path();
    let ok = OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)
        .and_then(|mut f| writeln!(f, "{text}"))
        .is_ok();
    if ok {
        ExecutionOutcome::Launched
    } else {
        ExecutionOutcome::Failed("could not write note".into())
    }
}
