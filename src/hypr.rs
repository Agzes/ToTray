use std::path::Path;
use std::process::Command;
use std::time::{Duration, Instant, SystemTime};

pub fn wait_for_session(timeout: Duration) -> bool {
    let deadline = Instant::now() + timeout;

    println!("[ToTray] Waiting for Hyprland to finish loading...");
    loop {
        if let Some(sig) = live_session() {
            if std::env::var("HYPRLAND_INSTANCE_SIGNATURE").ok().as_deref() != Some(sig.as_str()) {
                unsafe {
                    std::env::set_var("HYPRLAND_INSTANCE_SIGNATURE", &sig);
                }
            }
            println!("[ToTray] Hyprland is ready.");
            std::thread::sleep(Duration::from_secs(2));
            return true;
        }
        if Instant::now() >= deadline {
            eprintln!("[ToTray] Hyprland is not ready after {}s.", timeout.as_secs());
            return false;
        }
        std::thread::sleep(Duration::from_millis(500));
    }
}

fn live_session() -> Option<String> {
    let runtime = dirs::runtime_dir()?;
    session_candidates(&runtime)
        .into_iter()
        .map(|(_, sig)| sig)
        .find(|sig| instance_responds(sig))
}

fn instance_responds(sig: &str) -> bool {
    let Ok(out) = Command::new("hyprctl")
        .args(["-i", sig, "monitors", "-j"])
        .output()
    else {
        return false;
    };
    if !out.status.success() {
        return false;
    }
    let text = String::from_utf8_lossy(&out.stdout);
    let trimmed = text.trim();
    trimmed.starts_with('[') && trimmed != "[]"
}

fn is_session_dir(path: &Path) -> bool {
    path.join(".socket.sock").exists() || path.join(".socket2.sock").exists()
}

fn session_candidates(runtime: &Path) -> Vec<(SystemTime, String)> {
    let mut found = Vec::new();
    let Ok(entries) = std::fs::read_dir(runtime.join("hypr")) else {
        return found;
    };

    for entry in entries.flatten() {
        let path = entry.path();
        if !path.is_dir() || !is_session_dir(&path) {
            continue;
        }
        let mtime = entry
            .metadata()
            .and_then(|m| m.modified())
            .unwrap_or(SystemTime::UNIX_EPOCH);
        let name = entry.file_name().to_string_lossy().into_owned();
        found.push((mtime, name));
    }

    found.sort_by_key(|entry| std::cmp::Reverse(entry.0));
    found
}

pub fn exec(command: &str) -> bool {
    Command::new("hyprctl")
        .args(["dispatch", "exec", command])
        .output()
        .map(|out| out.status.success())
        .unwrap_or(false)
}

pub fn close_window(class: &str) {
    Command::new("hyprctl")
        .args(["dispatch", "closewindow", &format!("class:{}", class)])
        .spawn()
        .ok();
}

pub fn move_workspace(class: &str, ws: u32) {
    Command::new("hyprctl")
        .args([
            "dispatch",
            "movetoworkspacesilent",
            &format!("{},class:{}", ws, class),
        ])
        .spawn()
        .ok();
}

pub fn hide_to_special(class: &str) {
    Command::new("hyprctl")
        .args([
            "dispatch",
            "movetoworkspacesilent",
            &format!("special,class:{}", class),
        ])
        .spawn()
        .ok();
}

pub fn show_from_special(class: &str) {
    Command::new("hyprctl")
        .args(["dispatch", "movetoworkspace", &format!("+0,class:{}", class)])
        .spawn()
        .ok();
}

pub fn get_window_count(class: &str) -> usize {
    let output = Command::new("hyprctl").args(["clients"]).output();

    if let Ok(out) = output {
        let s = String::from_utf8_lossy(&out.stdout);

        s.split('\n')
            .filter(|l| l.contains(&format!("class: {}", class)))
            .count()
    } else {
        0
    }
}
