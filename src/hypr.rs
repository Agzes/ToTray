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
            wait_for_session_env(Duration::from_secs(10));
            println!("[ToTray] Hyprland is ready.");
            std::thread::sleep(Duration::from_secs(2));
            return true;
        }
        if Instant::now() >= deadline {
            eprintln!(
                "[ToTray] Hyprland is not ready after {}s.",
                timeout.as_secs()
            );
            return false;
        }
        std::thread::sleep(Duration::from_millis(500));
    }
}

fn wait_for_session_env(timeout: Duration) {
    let deadline = Instant::now() + timeout;
    loop {
        sync_systemd_env();
        ensure_wayland_display();
        ensure_x11_display();
        if std::env::var_os("WAYLAND_DISPLAY").is_some() && std::env::var_os("DISPLAY").is_some() {
            return;
        }
        if Instant::now() >= deadline {
            return;
        }
        std::thread::sleep(Duration::from_millis(500));
    }
}

fn sync_systemd_env() {
    let Ok(out) = Command::new("systemctl")
        .args(["--user", "show-environment"])
        .output()
    else {
        return;
    };
    if !out.status.success() {
        return;
    }
    let text = String::from_utf8_lossy(&out.stdout);
    for line in text.lines() {
        let Some((key, val)) = line.split_once('=') else {
            continue;
        };
        if matches!(
            key,
            "WAYLAND_DISPLAY"
                | "DISPLAY"
                | "XDG_CURRENT_DESKTOP"
                | "XDG_SESSION_TYPE"
                | "GTK_THEME"
                | "QT_QPA_PLATFORMTHEME"
                | "QT_STYLE_OVERRIDE"
        ) && std::env::var_os(key).is_none()
        {
            unsafe {
                std::env::set_var(key, val);
            }
        }
    }
}

fn ensure_x11_display() {
    if std::env::var_os("DISPLAY").is_some() {
        return;
    }
    let Ok(entries) = std::fs::read_dir("/tmp/.X11-unix") else {
        return;
    };
    for entry in entries.flatten() {
        let name = entry.file_name().to_string_lossy().into_owned();
        if let Some(num) = name.strip_prefix('X')
            && !num.is_empty()
        {
            unsafe {
                std::env::set_var("DISPLAY", format!(":{num}"));
            }
            return;
        }
    }
}

fn ensure_wayland_display() {
    if std::env::var_os("WAYLAND_DISPLAY").is_some() {
        return;
    }
    let Some(runtime) = dirs::runtime_dir() else {
        return;
    };
    let Ok(entries) = std::fs::read_dir(&runtime) else {
        return;
    };
    for entry in entries.flatten() {
        let name = entry.file_name().to_string_lossy().into_owned();
        if name.starts_with("wayland-") && !name.ends_with(".lock") {
            unsafe {
                std::env::set_var("WAYLAND_DISPLAY", &name);
            }
            return;
        }
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
    dispatch(&["exec", command]).is_ok()
}

fn lua_escape(s: &str) -> String {
    let mut out = String::with_capacity(s.len() + 2);
    out.push('"');
    for ch in s.chars() {
        match ch {
            '\\' => out.push_str("\\\\"),
            '"' => out.push_str("\\\""),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            _ => out.push(ch),
        }
    }
    out.push('"');
    out
}

fn dispatch_lua(args: &[&str]) -> Result<(), String> {
    let code = match args {
        ["exec", cmd] => format!("hl.dispatch(hl.dsp.exec_cmd({}))", lua_escape(cmd)),
        ["closewindow", win] => format!(
            "hl.dispatch(hl.dsp.window.close({{ window = {} }}))",
            lua_escape(win)
        ),
        ["setfloating", win] => format!(
            "hl.dispatch(hl.dsp.window.float({{ action = \"set\", window = {} }}))",
            lua_escape(win)
        ),
        ["focuswindow", win] => format!(
            "hl.dispatch(hl.dsp.window.focus({{ window = {} }}))",
            lua_escape(win)
        ),
        ["fullscreen", _] => {
            "hl.dispatch(hl.dsp.window.fullscreen({ mode = \"fullscreen\", action = \"set\" }))"
                .to_string()
        }
        ["fullscreen_window", win] => format!(
            "hl.dispatch(hl.dsp.window.fullscreen({{ mode = \"fullscreen\", action = \"set\", window = {} }}))",
            lua_escape(win)
        ),
        ["movetoworkspacesilent", target] => {
            let (ws, win) = target
                .split_once(',')
                .ok_or_else(|| format!("Invalid move target: {target}"))?;
            let ws_val = if ws.parse::<i64>().is_ok() {
                ws.to_string()
            } else {
                lua_escape(ws)
            };
            format!(
                "hl.dispatch(hl.dsp.window.move({{ workspace = {ws_val}, follow = false, window = {} }}))",
                lua_escape(win)
            )
        }
        ["movetoworkspace", target] => {
            let (ws, win) = target
                .split_once(',')
                .ok_or_else(|| format!("Invalid move target: {target}"))?;
            let ws_val = if ws.parse::<i64>().is_ok() {
                ws.to_string()
            } else {
                lua_escape(ws)
            };
            format!(
                "hl.dispatch(hl.dsp.window.move({{ workspace = {ws_val}, follow = true, window = {} }}))",
                lua_escape(win)
            )
        }
        _ => return Err(format!("Unsupported lua dispatcher: {}", args.join(" "))),
    };

    let out = Command::new("hyprctl")
        .args(["eval", &code])
        .output()
        .map_err(|e| format!("Failed to run hyprctl eval: {e}"))?;

    let stdout = String::from_utf8_lossy(&out.stdout).trim().to_string();
    let stderr = String::from_utf8_lossy(&out.stderr).trim().to_string();

    if !out.status.success() {
        let msg = if !stderr.is_empty() { stderr } else { stdout };
        return Err(format!("hyprctl eval failed: {msg}"));
    }

    if stdout.starts_with("error:") {
        return Err(format!("hyprctl eval: {stdout}"));
    }

    Ok(())
}

fn dispatch(args: &[&str]) -> Result<(), String> {
    let out = Command::new("hyprctl")
        .arg("dispatch")
        .args(args)
        .output()
        .map_err(|e| format!("Failed to run hyprctl: {e}"))?;

    let stdout = String::from_utf8_lossy(&out.stdout).trim().to_string();
    let stderr = String::from_utf8_lossy(&out.stderr).trim().to_string();

    if !out.status.success() {
        let msg = if !stderr.is_empty() { stderr } else { stdout };
        if msg.contains("hl.dispatch") || msg.contains("dispatch in lua") {
            return dispatch_lua(args);
        }
        return Err(format!("hyprctl failed: {msg}"));
    }

    if !stdout.is_empty() && stdout != "ok" {
        if stdout.contains("hl.dispatch") || stdout.contains("dispatch in lua") {
            return dispatch_lua(args);
        }
        return Err(format!("hyprctl: {stdout}"));
    }

    Ok(())
}

pub fn close_window(class: &str) -> Result<(), String> {
    dispatch(&["closewindow", &format!("class:{class}")])
}

pub fn set_floating(class: &str) -> Result<(), String> {
    dispatch(&["setfloating", &format!("class:{class}")])
}

pub fn fullscreen_window(class: &str) -> Result<(), String> {
    let target = format!("class:{class}");
    if dispatch_lua(&["fullscreen_window", &target]).is_ok() {
        return Ok(());
    }
    dispatch(&["focuswindow", &target])?;
    dispatch(&["fullscreenstate", "2", "0"])
}

pub fn move_workspace_target(target: &str, ws: u32) -> Result<(), String> {
    dispatch(&["movetoworkspacesilent", &format!("{ws},{target}")])
}

pub fn move_special_workspace_target(target: &str, name: &str) -> Result<(), String> {
    let trimmed = name.trim();
    let ws = trimmed.strip_prefix("special:").unwrap_or(trimmed);
    let spec = if ws.is_empty() {
        format!("special,{target}")
    } else {
        format!("special:{ws},{target}")
    };
    dispatch(&["movetoworkspacesilent", &spec])
}

pub fn hide_to_special_target(target: &str) -> Result<(), String> {
    dispatch(&["movetoworkspacesilent", &format!("special,{target}")])
}

fn active_workspace_id() -> Option<i64> {
    let out = Command::new("hyprctl")
        .args(["activeworkspace", "-j"])
        .output()
        .ok()?;
    if !out.status.success() {
        return None;
    }
    let val: serde_json::Value = serde_json::from_slice(&out.stdout).ok()?;
    val.get("id").and_then(|v| v.as_i64())
}

pub fn show_from_special(class: &str) {
    let target = active_workspace_id()
        .map(|id| format!("{id},class:{class}"))
        .unwrap_or_else(|| format!("+0,class:{class}"));
    let _ = dispatch(&["movetoworkspace", &target]);
}

pub fn list_open_classes() -> Vec<String> {
    let Ok(out) = Command::new("hyprctl").args(["clients", "-j"]).output() else {
        return Vec::new();
    };
    if !out.status.success() {
        return Vec::new();
    }
    let Ok(serde_json::Value::Array(clients)) = serde_json::from_slice(&out.stdout) else {
        return Vec::new();
    };
    let mut classes: Vec<String> = clients
        .iter()
        .filter_map(|c| c.get("class").and_then(|v| v.as_str()))
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(str::to_string)
        .collect();
    classes.sort_by_key(|a| a.to_lowercase());
    classes.dedup();
    classes
}

pub fn list_window_addresses(class: &str) -> Vec<String> {
    list_window_addresses_filtered(class, None)
}

pub fn list_window_addresses_outside(class: &str, target_ws: &str) -> Vec<String> {
    list_window_addresses_filtered(class, Some(target_ws))
}

fn is_on_workspace(client: &serde_json::Value, target_ws: &str) -> bool {
    let Some(ws) = client.get("workspace") else {
        return false;
    };
    let ws_name = ws.get("name").and_then(|v| v.as_str()).unwrap_or("");
    let ws_id = ws.get("id").and_then(|v| v.as_i64());

    if target_ws == "special" {
        return ws_name == "special" || ws_name == "special:special";
    }
    if let Ok(target_id) = target_ws.parse::<i64>() {
        return ws_id == Some(target_id) || ws_name == target_ws;
    }
    ws_name == target_ws
}

fn list_window_addresses_filtered(class: &str, outside_ws: Option<&str>) -> Vec<String> {
    let Ok(out) = Command::new("hyprctl").args(["clients", "-j"]).output() else {
        return Vec::new();
    };
    if !out.status.success() {
        return Vec::new();
    }
    let Ok(serde_json::Value::Array(clients)) = serde_json::from_slice(&out.stdout) else {
        return Vec::new();
    };

    let addr_of = |c: &serde_json::Value| {
        if let Some(ws) = outside_ws
            && is_on_workspace(c, ws)
        {
            return None;
        }
        c.get("address")
            .and_then(|v| v.as_str())
            .map(str::to_string)
    };

    let lower = class.to_lowercase();
    let ci_exact: Vec<String> = clients
        .iter()
        .filter(|c| {
            let cls = c
                .get("class")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_lowercase();
            let init = c
                .get("initialClass")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_lowercase();
            cls == lower || init == lower
        })
        .filter_map(addr_of)
        .collect();
    if !ci_exact.is_empty() {
        return ci_exact;
    }

    clients
        .iter()
        .filter(|c| {
            let cls = c
                .get("class")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_lowercase();
            let init = c
                .get("initialClass")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_lowercase();
            cls.contains(&lower) || init.contains(&lower)
        })
        .filter_map(addr_of)
        .collect()
}

pub fn get_window_count(class: &str) -> usize {
    list_window_addresses(class).len()
}
