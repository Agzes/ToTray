use crate::hypr;
use crate::state::{APP_ID, APP_SLUG, APP_TITLE, Action, AppRule, SharedState, push_log};
use notify_rust::Notification;
use std::io::{BufRead, BufReader};
use std::process::{Command, Stdio};
use std::thread;
use std::time::Duration;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum DesktopIntegrationStatus {
    Aur { package: String, helper: String },
    Installed { up_to_date: bool },
    NotInstalled,
}

pub fn detect_aur_package() -> Option<String> {
    if let Ok(current_exe) = std::env::current_exe() {
        if let Ok(output) = Command::new("pacman")
            .env("LC_ALL", "C")
            .args(["-Qo", &current_exe.to_string_lossy()])
            .output()
            && output.status.success()
        {
            let stdout = String::from_utf8_lossy(&output.stdout);
            let words: Vec<&str> = stdout.split_whitespace().collect();
            if let Some(pos) = words.iter().position(|w| *w == "by")
                && let Some(pkg) = words.get(pos + 1)
            {
                return Some(pkg.to_string());
            }
            if stdout.contains(APP_SLUG) {
                return Some(APP_SLUG.to_string());
            }
        }

        if current_exe.starts_with("/usr")
            && let Ok(output) = Command::new("pacman")
                .env("LC_ALL", "C")
                .args(["-Q", APP_SLUG])
                .output()
            && output.status.success()
        {
            return Some(APP_SLUG.to_string());
        }
    }

    None
}

pub fn detect_aur_helper() -> String {
    for helper in ["yay", "paru"] {
        if Command::new(helper)
            .arg("--version")
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false)
        {
            return helper.to_string();
        }
    }
    "yay".to_string()
}

pub fn start_backend(state: SharedState) {
    let (apps, delay, notify, multi) = {
        let s = state.lock().unwrap();
        (
            s.apps.clone(),
            s.launch_delay,
            s.notifications,
            s.multi_launch,
        )
    };

    println!("[ToTray] {} rule(s) loaded.", apps.len());
    if apps.is_empty() {
        eprintln!("[ToTray] No rules configured, nothing to launch.");
        return;
    }

    if multi {
        for app in apps {
            let state_c = state.clone();
            thread::spawn(move || {
                run_rule(&app, delay, notify, &state_c);
            });
        }
    } else {
        thread::spawn(move || {
            for app in apps {
                run_rule(&app, delay, notify, &state);
            }
        });
    }
}

pub fn run_rule(app: &AppRule, delay: u64, notify: bool, state: &SharedState) {
    {
        let mut s = state.lock().unwrap();
        s.running_rules += 1;
    }

    if notify
        && let Err(e) = Notification::new()
            .summary(APP_TITLE)
            .body(&format!("Launching {}", app.name))
            .icon(APP_SLUG)
            .show()
    {
        eprintln!("[ToTray] Notification failed: {e}");
    }

    println!("[ToTray] Launching {} ({})", app.name, app.exec);
    push_log(
        state,
        &app.name,
        format!("[ToTray] Launching: {}", app.exec),
    );

    let has_full_display =
        std::env::var_os("WAYLAND_DISPLAY").is_some() && std::env::var_os("DISPLAY").is_some();

    let launched = if hypr::exec(&app.exec) {
        push_log(state, &app.name, "[ToTray] Launched via Hyprland.");
        true
    } else if has_full_display {
        launch_captured(app, state.clone())
    } else {
        false
    };

    if !launched {
        push_log(
            state,
            &app.name,
            "[ToTray] Error: failed to launch command.",
        );
        let mut s = state.lock().unwrap();
        s.running_rules = s.running_rules.saturating_sub(1);
        return;
    }

    if matches!(app.action, Action::LaunchOnly) {
        push_log(state, &app.name, "[ToTray] Launched (no window action).");
        let mut s = state.lock().unwrap();
        s.running_rules = s.running_rules.saturating_sub(1);
        return;
    }

    match app.action {
        Action::Close => close_windows(&app.name, 1, delay, state),
        Action::Close2 => close_windows(&app.name, 2, delay, state),
        Action::CloseTimes(count) => close_windows(&app.name, count.max(1), delay, state),
        Action::Workspace(ws) => {
            move_windows(
                &app.name,
                1,
                delay,
                state,
                &ws.to_string(),
                &format!("Moving window to workspace {ws}"),
                |target| hypr::move_workspace_target(target, ws),
            );
        }
        Action::WorkspaceTimes(ws, count) => {
            move_windows(
                &app.name,
                count.max(1),
                delay,
                state,
                &ws.to_string(),
                &format!("Moving window to workspace {ws}"),
                |target| hypr::move_workspace_target(target, ws),
            );
        }
        Action::SpecialWorkspace(ref name) => {
            let label = special_label(name);
            move_windows(
                &app.name,
                1,
                delay,
                state,
                &label,
                &format!("Moving window to {label}"),
                |target| hypr::move_special_workspace_target(target, name),
            );
        }
        Action::SpecialWorkspaceTimes(ref name, count) => {
            let label = special_label(name);
            move_windows(
                &app.name,
                count.max(1),
                delay,
                state,
                &label,
                &format!("Moving window to {label}"),
                |target| hypr::move_special_workspace_target(target, name),
            );
        }
        Action::HideToTray => hide_windows(&app.name, 1, delay, state),
        Action::HideToTrayTimes(count) => hide_windows(&app.name, count.max(1), delay, state),
        Action::Float => {
            if wait_for_window(&app.name, delay, state) {
                push_log(state, &app.name, "[ToTray] Setting window floating.");
                if let Err(e) = hypr::set_floating(&app.name) {
                    push_log(state, &app.name, format!("[ERR] {e}"));
                }
            }
        }
        Action::Fullscreen => {
            if wait_for_window(&app.name, delay, state) {
                push_log(state, &app.name, "[ToTray] Setting window fullscreen.");
                if let Err(e) = hypr::fullscreen_window(&app.name) {
                    push_log(state, &app.name, format!("[ERR] {e}"));
                }
            }
        }
        Action::LaunchOnly => {}
    }

    let mut s = state.lock().unwrap();
    s.running_rules = s.running_rules.saturating_sub(1);
}

fn special_label(name: &str) -> String {
    let ws = name.trim().strip_prefix("special:").unwrap_or(name.trim());
    if ws.is_empty() {
        "special".to_string()
    } else {
        format!("special:{ws}")
    }
}

fn hide_windows(class: &str, count: u32, delay: u64, state: &SharedState) {
    let ok = move_windows(
        class,
        count,
        delay,
        state,
        "special",
        "Hiding window to special tray",
        hypr::hide_to_special_target,
    );
    if ok {
        let mut s = state.lock().unwrap();
        if !s.hidden_apps.iter().any(|a| a == class) {
            s.hidden_apps.push(class.to_string());
        }
    }
}

fn move_windows<F>(
    class: &str,
    count: u32,
    delay: u64,
    state: &SharedState,
    target_ws: &str,
    label: &str,
    mut op: F,
) -> bool
where
    F: FnMut(&str) -> Result<(), String>,
{
    let mut any_success = false;

    for i in 1..=count {
        let max_iters = if i == 1 { 60 } else { 80 };
        let mut next_addr: Option<String> = None;

        for _ in 0..max_iters {
            let addrs = hypr::list_window_addresses_outside(class, target_ws);
            if let Some(addr) = addrs.into_iter().next() {
                next_addr = Some(addr);
                break;
            }
            thread::sleep(Duration::from_millis(500));
        }

        let Some(addr) = next_addr else {
            push_log(
                state,
                class,
                format!("[ToTray] Timed out waiting for window class '{class}' ({i}/{count})."),
            );
            break;
        };

        if i == 1 && delay > 0 {
            thread::sleep(Duration::from_millis(delay));
        }

        if count > 1 {
            push_log(
                state,
                class,
                format!("[ToTray] {label} ({i}/{count}) [{addr}]."),
            );
        } else {
            push_log(state, class, format!("[ToTray] {label}."));
        }

        let target = format!("address:{addr}");
        match op(&target) {
            Ok(()) => any_success = true,
            Err(e) => push_log(state, class, format!("[ERR] {e}")),
        }

        if i < count {
            thread::sleep(Duration::from_millis(500));
        }
    }

    any_success
}

fn wait_for_window(class: &str, delay: u64, state: &SharedState) -> bool {
    for _ in 0..60 {
        if hypr::get_window_count(class) > 0 {
            if delay > 0 {
                thread::sleep(Duration::from_millis(delay));
            }
            return true;
        }
        thread::sleep(Duration::from_millis(500));
    }
    push_log(
        state,
        class,
        format!("[ToTray] Timed out waiting for window class '{class}'."),
    );
    false
}

fn close_windows(class: &str, count: u32, delay: u64, state: &SharedState) {
    for i in 1..=count {
        let max_iters = if i == 1 { 40 } else { 80 };
        let mut closed = false;
        for _ in 0..max_iters {
            let current = hypr::get_window_count(class);
            if current > 0 {
                if i == 1 && delay > 0 {
                    thread::sleep(Duration::from_millis(delay));
                }
                push_log(
                    state,
                    class,
                    format!("[ToTray] Closing window ({i}/{count})."),
                );
                if let Err(e) = hypr::close_window(class) {
                    push_log(state, class, format!("[ERR] {e}"));
                }
                closed = true;

                if i < count {
                    for _ in 0..25 {
                        thread::sleep(Duration::from_millis(200));
                        if hypr::get_window_count(class) < current {
                            break;
                        }
                    }
                    thread::sleep(Duration::from_millis(500));
                }
                break;
            }
            thread::sleep(Duration::from_millis(500));
        }

        if !closed {
            push_log(
                state,
                class,
                format!("[ToTray] Timed out waiting for window class '{class}' ({i}/{count})."),
            );
            break;
        }
    }
}

pub fn launch_captured(app: &AppRule, state: SharedState) -> bool {
    let exec = app.exec.clone();
    let name = app.name.clone();

    let child = Command::new("sh")
        .args(["-c", &exec])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn();

    let mut c = match child {
        Ok(c) => c,
        Err(e) => {
            eprintln!("[ToTray] Failed to launch {}: {}", name, e);
            push_log(&state, &name, format!("[ToTray] Spawn error: {e}"));
            return false;
        }
    };

    if let Some(stdout) = c.stdout.take() {
        capture_output(stdout, state.clone(), name.clone(), "");
    }
    if let Some(stderr) = c.stderr.take() {
        capture_output(stderr, state.clone(), name.clone(), "[ERR] ");
    }

    thread::spawn(move || {
        let _ = c.wait();
    });

    true
}

fn capture_output(
    reader: impl std::io::Read + Send + 'static,
    state: SharedState,
    name: String,
    prefix: &'static str,
) {
    thread::spawn(move || {
        for line in BufReader::new(reader).lines().map_while(Result::ok) {
            push_log(&state, &name, format!("{prefix}{line}"));
        }
    });
}

const SERVICE_NAME: &str = "totray.service";

fn service_unit_path() -> Option<std::path::PathBuf> {
    let mut dir = dirs::config_dir()?;
    dir.push("systemd/user");
    let _ = std::fs::create_dir_all(&dir);
    Some(dir.join(SERVICE_NAME))
}

fn install_self(target: &std::path::Path) -> std::io::Result<()> {
    use std::os::unix::fs::PermissionsExt;
    if let Some(parent) = target.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let tmp = target.with_extension("tmp");
    std::fs::copy("/proc/self/exe", &tmp)?;
    std::fs::set_permissions(&tmp, std::fs::Permissions::from_mode(0o755))?;
    std::fs::rename(&tmp, target)?;
    Ok(())
}

fn clean_current_exe() -> Option<std::path::PathBuf> {
    let exe = std::env::current_exe().ok()?;
    let raw = exe.to_string_lossy().into_owned();
    let path = match raw.strip_suffix(" (deleted)") {
        Some(stripped) => std::path::PathBuf::from(stripped),
        None => exe,
    };
    path.exists().then_some(path)
}

fn resolve_bin() -> Option<std::path::PathBuf> {
    if detect_aur_package().is_some()
        && let Some(exe) = clean_current_exe()
    {
        return Some(exe);
    }
    let local = dirs::home_dir()?.join(".local/bin/totray");
    if local.exists() {
        return Some(local);
    }
    if install_self(&local).is_ok() {
        return Some(local);
    }
    clean_current_exe()
}

fn systemctl(args: &[&str]) -> bool {
    match Command::new("systemctl").args(args).status() {
        Ok(status) => status.success(),
        Err(e) => {
            eprintln!("[ToTray] Failed to run systemctl: {e}");
            false
        }
    }
}

fn service_enabled() -> bool {
    Command::new("systemctl")
        .args(["--user", "--quiet", "is-enabled", SERVICE_NAME])
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}

pub fn autostart(add: bool) {
    remove_hyprland_autostart();

    let Some(path) = service_unit_path() else {
        eprintln!("[ToTray] Cannot resolve systemd user unit directory.");
        return;
    };

    if add {
        if !enable_service(true) {
            eprintln!("[ToTray] Failed to enable systemd user service.");
        }
    } else {
        let _ = systemctl(&["--user", "disable", "--now", SERVICE_NAME]);
        if path.exists() {
            let _ = std::fs::remove_file(&path);
        }
        let _ = systemctl(&["--user", "daemon-reload"]);
    }
}

pub fn migrate_legacy_autostart(auto_start: bool) {
    let had_legacy = remove_hyprland_autostart();
    if !auto_start || !had_legacy || service_enabled() {
        return;
    }
    if enable_service(false) {
        println!(
            "[ToTray] Autostart migrated to systemd user service \
             (active from the next login)."
        );
    }
}

fn enable_service(start_now: bool) -> bool {
    if !write_service_unit() {
        return false;
    }
    if !systemctl(&["--user", "daemon-reload"]) {
        return false;
    }
    if start_now {
        systemctl(&["--user", "enable", "--now", SERVICE_NAME])
    } else {
        systemctl(&["--user", "enable", SERVICE_NAME])
    }
}

fn write_service_unit() -> bool {
    let Some(path) = service_unit_path() else {
        return false;
    };
    let Some(bin) = resolve_bin() else {
        eprintln!("[ToTray] Could not resolve the totray binary path.");
        return false;
    };
    let content = format!(
        "[Unit]\n\
         Description=ToTray - Autorun and Tray Manager for Hyprland\n\
         After=graphical-session.target\n\
         PartOf=graphical-session.target\n\
         \n\
         [Service]\n\
         Type=simple\n\
         ExecStart=\"{}\" --worker\n\
         Restart=on-failure\n\
         RestartSec=3\n\
         \n\
         [Install]\n\
         WantedBy=default.target\n",
        bin.display()
    );
    match std::fs::write(&path, content) {
        Ok(()) => true,
        Err(e) => {
            eprintln!("[ToTray] Failed to write {}: {e}", path.display());
            false
        }
    }
}

fn remove_hyprland_autostart() -> bool {
    let Some(config_dir) = dirs::config_dir() else {
        return false;
    };
    let hypr_conf = config_dir.join("hypr/hyprland.conf");
    let Ok(content) = std::fs::read_to_string(&hypr_conf) else {
        return false;
    };

    let mut lines: Vec<String> = content.lines().map(|s| s.to_string()).collect();
    lines.retain(|l| !l.contains("totray") && !l.contains("# ToTray:"));
    while lines.last().map(|l| l.trim().is_empty()).unwrap_or(false) {
        lines.pop();
    }
    let new_content = lines.join("\n") + "\n";

    if content == new_content {
        return false;
    }
    let _ = std::fs::write(&hypr_conf, new_content);
    true
}

pub fn desktop_file_exists() -> bool {
    if detect_aur_package().is_some() {
        return true;
    }
    for sys_dir in ["/usr/share/applications", "/usr/local/share/applications"] {
        let base = std::path::Path::new(sys_dir);
        if base.join("totray.desktop").exists() || base.join(format!("{APP_ID}.desktop")).exists() {
            return true;
        }
    }
    if let Some(data_dir) = dirs::data_local_dir() {
        return data_dir.join("applications/totray.desktop").exists()
            || data_dir
                .join(format!("applications/{APP_ID}.desktop"))
                .exists();
    }
    false
}

pub fn check_desktop_integration() -> DesktopIntegrationStatus {
    if let Some(package) = detect_aur_package() {
        return DesktopIntegrationStatus::Aur {
            package,
            helper: detect_aur_helper(),
        };
    }

    let Some(home) = dirs::home_dir() else {
        return DesktopIntegrationStatus::NotInstalled;
    };

    let apps_dir = home.join(".local/share/applications");
    let desktop_exists = apps_dir.join("totray.desktop").exists()
        || apps_dir.join(format!("{APP_ID}.desktop")).exists();
    let target_bin = home.join(".local/bin").join(APP_SLUG);

    if desktop_exists && target_bin.exists() {
        if let Ok(current_exe) = std::env::current_exe() {
            if current_exe == target_bin || files_match(&current_exe, &target_bin) {
                DesktopIntegrationStatus::Installed { up_to_date: true }
            } else {
                DesktopIntegrationStatus::Installed { up_to_date: false }
            }
        } else {
            DesktopIntegrationStatus::Installed { up_to_date: true }
        }
    } else {
        DesktopIntegrationStatus::NotInstalled
    }
}

pub fn is_in_path() -> bool {
    if let Ok(path) = std::env::var("PATH") {
        let bin_dir = dirs::home_dir()
            .map(|h| h.join(".local/bin"))
            .unwrap_or_default();
        let bin_dir_str = bin_dir.to_string_lossy();

        return path.split(':').any(|p| {
            let p_path = std::path::Path::new(p);
            p == bin_dir_str
                || p_path == bin_dir
                || p_path.canonicalize().ok() == bin_dir.canonicalize().ok()
        });
    }
    false
}

pub fn install_desktop_entry() -> Result<(), String> {
    let data = dirs::data_local_dir().ok_or("Local data directory not found")?;
    let home = dirs::home_dir().ok_or("Home directory not found")?;

    let icons_dir_png = data.join("icons/hicolor/256x256/apps");
    let _ = std::fs::create_dir_all(&icons_dir_png);
    let logo_bytes = include_bytes!("../assets/logo.png");
    let _ = std::fs::write(icons_dir_png.join("totray.png"), logo_bytes);
    let _ = std::fs::write(icons_dir_png.join(format!("{APP_ID}.png")), logo_bytes);

    let bin_dir = home.join(".local/bin");
    std::fs::create_dir_all(&bin_dir)
        .map_err(|e| format!("cannot create {}: {e}", bin_dir.display()))?;
    let target_bin = bin_dir.join(APP_SLUG);

    if clean_current_exe().as_deref() != Some(target_bin.as_path()) {
        install_self(&target_bin)
            .map_err(|e| format!("cannot install binary to {}: {e}", target_bin.display()))?;
    }

    let apps_dir = data.join("applications");
    std::fs::create_dir_all(&apps_dir)
        .map_err(|e| format!("cannot create {}: {e}", apps_dir.display()))?;
    let desktop_path = apps_dir.join("totray.desktop");

    let content = format!(
        "[Desktop Entry]\n\
         Name={APP_TITLE}\n\
         Comment=Autorun and Tray Manager for Hyprland\n\
         Exec={}\n\
         Icon=totray\n\
         Terminal=false\n\
         Type=Application\n\
         Categories=Utility;System;\n\
         StartupNotify=false\n\
         StartupWMClass={APP_ID}\n\
         Actions=Settings;\n\n\
         [Desktop Action Settings]\n\
         Name=Settings\n\
         Exec={}\n",
        target_bin.display(),
        target_bin.display()
    );
    std::fs::write(&desktop_path, &content)
        .map_err(|e| format!("cannot write {}: {e}", desktop_path.display()))?;

    let _ = Command::new("update-desktop-database")
        .arg(&apps_dir)
        .output();

    Ok(())
}

pub fn uninstall_desktop_entry() -> Result<(), String> {
    let home = dirs::home_dir().ok_or("Home directory not found")?;
    let data = dirs::data_local_dir().ok_or("Local data directory not found")?;

    let bin_path = home.join(".local/bin").join(APP_SLUG);
    if bin_path.exists() {
        let _ = std::fs::remove_file(&bin_path);
    }

    let apps_dir = data.join("applications");
    for name in ["totray.desktop", &format!("{APP_ID}.desktop")] {
        let p = apps_dir.join(name);
        if p.exists() {
            let _ = std::fs::remove_file(p);
        }
    }

    if let Some(user_desktop) = dirs::desktop_dir() {
        let shortcut_path = user_desktop.join("totray.desktop");
        if shortcut_path.exists() {
            let _ = std::fs::remove_file(shortcut_path);
        }
    }

    let icons_dir = data.join("icons/hicolor/256x256/apps");
    for name in ["totray.png", &format!("{APP_ID}.png")] {
        let p = icons_dir.join(name);
        if p.exists() {
            let _ = std::fs::remove_file(p);
        }
    }

    let _ = Command::new("update-desktop-database")
        .arg(&apps_dir)
        .output();

    Ok(())
}

pub fn add_to_path_config() -> bool {
    let home = match dirs::home_dir() {
        Some(h) => h,
        None => return false,
    };

    let bin_path_str = home.join(".local/bin").to_string_lossy().to_string();
    let mut success = false;

    let fish_config = home.join(".config/fish/config.fish");
    if fish_config.exists()
        && let Ok(content) = std::fs::read_to_string(&fish_config)
    {
        if !content.contains(".local/bin") {
            if let Ok(mut file) = std::fs::OpenOptions::new().append(true).open(&fish_config) {
                use std::io::Write;
                let _ = writeln!(file, "\n# Added by ToTray\nfish_add_path {bin_path_str}\n");
                success = true;
            }
        } else {
            success = true;
        }
    }

    let line = "\n# Added by ToTray\n\
         if [ -n \"$BASH_VERSION\" ] || [ -n \"$ZSH_VERSION\" ] || [ \"$SHELL\" != \"/usr/bin/fish\" ]; then\n  \
           export PATH=\"$HOME/.local/bin:$PATH\"\n\
         fi\n"
        .to_string();
    let shells = [".bashrc", ".zshrc", ".profile", ".bash_profile"];
    for shell in shells {
        let path = home.join(shell);
        if path.exists()
            && let Ok(content) = std::fs::read_to_string(&path)
        {
            if !content.contains(".local/bin") {
                if let Ok(mut file) = std::fs::OpenOptions::new().append(true).open(&path) {
                    use std::io::Write;
                    let _ = writeln!(file, "{line}");
                    success = true;
                }
            } else {
                success = true;
            }
        }
    }

    let config_dir = home.join(".config");
    if config_dir.exists() {
        let env_d = config_dir.join("environment.d");
        let _ = std::fs::create_dir_all(&env_d);
        let env_file = env_d.join("10-totray.conf");
        if let Ok(mut file) = std::fs::File::create(env_file) {
            use std::io::Write;
            let _ = writeln!(file, "PATH=\"$HOME/.local/bin:$PATH\"");
            success = true;
        }
    }

    success
}

pub fn files_match(p1: &std::path::Path, p2: &std::path::Path) -> bool {
    let mut f1 = match std::fs::File::open(p1) {
        Ok(f) => f,
        Err(_) => return false,
    };
    let mut f2 = match std::fs::File::open(p2) {
        Ok(f) => f,
        Err(_) => return false,
    };

    let m1 = f1.metadata().ok();
    let m2 = f2.metadata().ok();

    if let (Some(m1), Some(m2)) = (m1, m2)
        && m1.len() != m2.len()
    {
        return false;
    }

    use std::io::Read;
    let mut b1 = [0; 8192];
    let mut b2 = [0; 8192];

    loop {
        let n1 = f1.read(&mut b1).unwrap_or(0);
        let n2 = f2.read(&mut b2).unwrap_or(0);

        if n1 != n2 {
            return false;
        }
        if n1 == 0 {
            break;
        }
        if b1[..n1] != b2[..n1] {
            return false;
        }
    }

    true
}

pub fn is_hyprland() -> bool {
    std::env::var("HYPRLAND_INSTANCE_SIGNATURE").is_ok()
}

pub fn uninstall() -> bool {
    let Some(home) = dirs::home_dir() else {
        return false;
    };

    let _ = uninstall_desktop_entry();
    autostart(false);

    let shells = [
        ".bashrc",
        ".zshrc",
        ".profile",
        ".bash_profile",
        ".config/fish/config.fish",
    ];
    for shell in shells {
        let path = home.join(shell);
        if path.exists()
            && let Ok(content) = std::fs::read_to_string(&path)
        {
            let lines: Vec<String> = content
                .lines()
                .filter(|l| {
                    !l.contains("# Added by ToTray")
                        && !l.contains("totray")
                        && !l.contains("fish_add_path")
                })
                .map(|s| s.to_string())
                .collect();
            let _ = std::fs::write(&path, lines.join("\n") + "\n");
        }
    }

    let env_file = home.join(".config/environment.d/10-totray.conf");
    if env_file.exists() {
        let _ = std::fs::remove_file(env_file);
    }

    if let Some(config_dir) = dirs::config_dir() {
        let totray_config = config_dir.join(APP_SLUG);
        if totray_config.exists() {
            let _ = std::fs::remove_dir_all(totray_config);
        }
    }

    true
}
