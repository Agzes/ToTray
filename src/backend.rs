use crate::hypr;
use crate::state::{Action, AppRule, SharedState};
use notify_rust::Notification;
use std::io::{BufRead, BufReader};
use std::process::{Command, Stdio};
use std::thread;
use std::time::Duration;

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
                run_rule(&app, delay, notify, &state.clone());
            }
        });
    }
}

pub fn run_rule(app: &AppRule, delay: u64, notify: bool, state: &SharedState) {
    if notify
        && let Err(e) = Notification::new()
            .summary("ToTray")
            .body(&format!("Launching {}", app.name))
            .icon("totray")
            .show()
    {
        eprintln!("[ToTray] Notification failed: {}", e);
    }

    println!("[ToTray] Launching {} ({})", app.name, app.exec);
    if !hypr::exec(&app.exec) {
        eprintln!("[ToTray] hyprctl exec failed, falling back to a direct spawn.");
        launch_captured(app, state.clone());
    }

    if app.action != Action::Close2 {
        let mut found = false;
        for _ in 0..60 {
            if hypr::get_window_count(&app.name) > 0 {
                found = true;
                break;
            }
            thread::sleep(Duration::from_millis(500));
        }

        if !found {
            return;
        }

        if delay > 0 {
            thread::sleep(Duration::from_millis(delay));
        }
    }

    match app.action {
        Action::Close => hypr::close_window(&app.name),
        Action::Close2 => {
            let mut found = false;
            for _ in 0..40 {
                if hypr::get_window_count(&app.name) > 0 {
                    if delay > 0 {
                        thread::sleep(Duration::from_millis(delay));
                    }
                    hypr::close_window(&app.name);
                    found = true;
                    break;
                }
                thread::sleep(Duration::from_millis(500));
            }

            if found {
                thread::sleep(Duration::from_millis(1000));
                for _ in 0..80 {
                    if hypr::get_window_count(&app.name) > 0 {
                        hypr::close_window(&app.name);
                        break;
                    }
                    thread::sleep(Duration::from_millis(500));
                }
            }
        }
        Action::Workspace(ws) => hypr::move_workspace(&app.name, ws),
        Action::HideToTray => {
            hypr::hide_to_special(&app.name);
            let mut s = state.lock().unwrap();
            if !s.hidden_apps.contains(&app.name) {
                s.hidden_apps.push(app.name.clone());
            }
        }
    }
}

pub fn launch_captured(app: &AppRule, state: SharedState) {
    let exec = app.exec.clone();
    let name = app.name.clone();

    thread::spawn(move || {
        let child = Command::new("sh")
            .args(["-c", &exec])
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn();

        let mut c = match child {
            Ok(c) => c,
            Err(e) => {
                eprintln!("[ToTray] Failed to launch {}: {}", name, e);
                return;
            }
        };

        let stdout = c.stdout.take().unwrap();
        let stderr = c.stderr.take().unwrap();
        capture_output(stdout, state.clone(), name.clone(), "");
        capture_output(stderr, state.clone(), name.clone(), "[ERR] ");

        let _ = c.wait();
    });
}

fn capture_output(
    reader: impl std::io::Read + Send + 'static,
    state: SharedState,
    name: String,
    prefix: &'static str,
) {
    thread::spawn(move || {
        for line in BufReader::new(reader).lines().map_while(Result::ok) {
            let mut s = state.lock().unwrap();
            let entry = s.logs.entry(name.clone()).or_default();
            entry.push(format!("{}{}", prefix, line));
            if entry.len() > 500 {
                entry.remove(0);
            }
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
            eprintln!("[ToTray] Failed to run systemctl: {}", e);
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
            eprintln!("[ToTray] Failed to write {}: {}", path.display(), e);
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
    if let Some(data_dir) = dirs::data_local_dir() {
        let path = data_dir.join("applications/totray.desktop");
        return path.exists();
    }
    false
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

pub fn setup_desktop_file() -> bool {
    let data_dir = dirs::data_local_dir();
    let home_dir = dirs::home_dir();

    if let (Some(data), Some(home)) = (data_dir, home_dir) {
        let icons_dir_png = data.join("icons/hicolor/256x256/apps");
        if let Err(e) = std::fs::create_dir_all(&icons_dir_png) {
            eprintln!("Failed to create icons directory: {}", e);
        }
        let icon_path_png = icons_dir_png.join("totray.png");

        let logo_bytes = include_bytes!("../assets/logo.png");
        if let Err(e) = std::fs::write(&icon_path_png, logo_bytes) {
            eprintln!("Failed to write icon file: {}", e);
        }

        let bin_dir = home.join(".local/bin");
        if let Err(e) = std::fs::create_dir_all(&bin_dir) {
            eprintln!(
                "Failed to create bin directory {}: {}",
                bin_dir.display(),
                e
            );
        }
        let target_bin = bin_dir.join("totray");

        match install_self(&target_bin) {
            Ok(()) => {
                println!("Successfully installed binary to {}", target_bin.display());
            }
            Err(e) => {
                eprintln!(
                    "Failed to install binary to {}: {}",
                    target_bin.display(),
                    e
                );
            }
        }

        let apps_dir = data.join("applications");
        if let Err(e) = std::fs::create_dir_all(&apps_dir) {
            eprintln!("Failed to create applications directory: {}", e);
        }
        let desktop_path = apps_dir.join("totray.desktop");

        let content = format!(
            "[Desktop Entry]\n\
             Name=ToTray\n\
             Comment=Autorun and Tray Manager for Hyprland\n\
             Exec={}\n\
             Icon=totray\n\
             Terminal=false\n\
             Type=Application\n\
             Categories=Utility;System;\n\
             StartupNotify=false\n\
             Actions=Settings;\n\n\
             [Desktop Action Settings]\n\
             Name=Settings\n\
             Exec={}\n",
            target_bin.display(),
            target_bin.display()
        );
        match std::fs::write(&desktop_path, &content) {
            Ok(_) => {
                println!(
                    "Successfully created desktop entry at {}",
                    desktop_path.display()
                );

                if let Some(user_desktop) = dirs::desktop_dir() {
                    let shortcut_path = user_desktop.join("totray.desktop");
                    let _ = std::fs::write(&shortcut_path, &content);
                    #[cfg(unix)]
                    {
                        use std::os::unix::fs::PermissionsExt;
                        let _ = std::fs::set_permissions(
                            &shortcut_path,
                            std::fs::Permissions::from_mode(0o755),
                        );
                    }
                }

                return true;
            }
            Err(e) => {
                eprintln!(
                    "Failed to write desktop file {}: {}",
                    desktop_path.display(),
                    e
                );
                return false;
            }
        }
    }
    false
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
                let _ = writeln!(
                    file,
                    "\n# Added by ToTray\nfish_add_path {}\n",
                    bin_path_str
                );
                success = true;
            }
        } else {
            success = true;
        }
    }

    let line = "\n# Added by ToTray\n\
         if [ -n \"$BASH_VERSION\" ] || [ -n \"$ZSH_VERSION\" ] || [ \"$SHELL\" != \"/usr/bin/fish\" ]; then\n  \
           export PATH=\"$HOME/.local/bin:$PATH\"\n\
         fi\n".to_string();
    let shells = [".bashrc", ".zshrc", ".profile", ".bash_profile"];
    for shell in shells {
        let path = home.join(shell);
        if path.exists()
            && let Ok(content) = std::fs::read_to_string(&path)
        {
            if !content.contains(".local/bin") {
                if let Ok(mut file) = std::fs::OpenOptions::new().append(true).open(&path) {
                    use std::io::Write;
                    let _ = writeln!(file, "{}", line);
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

pub fn sync_binary() {
    let Some(home) = dirs::home_dir() else {
        return;
    };
    let target_bin = home.join(".local/bin/totray");

    if files_match(std::path::Path::new("/proc/self/exe"), &target_bin) {
        return;
    }

    println!("[ToTray] Updating binary in .local/bin...");
    if let Err(e) = install_self(&target_bin) {
        eprintln!("[ToTray] Failed to update binary: {}", e);
    }
}

fn files_match(p1: &std::path::Path, p2: &std::path::Path) -> bool {
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
    let home = match dirs::home_dir() {
        Some(h) => h,
        None => return false,
    };
    let data = match dirs::data_local_dir() {
        Some(d) => d,
        None => return false,
    };

    let bin_path = home.join(".local/bin/totray");
    if bin_path.exists() {
        let _ = std::fs::remove_file(bin_path);
    }

    let desktop_path = data.join("applications/totray.desktop");
    if desktop_path.exists() {
        let _ = std::fs::remove_file(desktop_path);
    }

    if let Some(user_desktop) = dirs::desktop_dir() {
        let shortcut_path = user_desktop.join("totray.desktop");
        if shortcut_path.exists() {
            let _ = std::fs::remove_file(shortcut_path);
        }
    }

    let icon_path = data.join("icons/hicolor/256x256/apps/totray.png");
    if icon_path.exists() {
        let _ = std::fs::remove_file(icon_path);
    }

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
        let totray_config = config_dir.join("totray");
        if totray_config.exists() {
            let _ = std::fs::remove_dir_all(totray_config);
        }
    }

    true
}
