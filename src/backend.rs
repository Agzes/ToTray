use crate::hypr;
use crate::state::{Action, AppRule, SharedState};
use notify_rust::Notification;
use std::io::{BufRead, BufReader};
use std::process::{Command, Stdio};
use std::thread;
use std::time::Duration;

pub fn start_backend(state: SharedState) {
    let (apps, delay, notify) = {
        let s = state.lock().unwrap();
        (s.apps.clone(), s.launch_delay, s.notifications)
    };

    for app in apps {
        let state_c = state.clone();
        thread::spawn(move || {
            run_rule(&app, delay, notify, &state_c);
        });
    }
}

pub fn run_rule(app: &AppRule, delay: u64, notify: bool, state: &SharedState) {
    if notify {
        let _ = Notification::new()
            .summary("ToTray")
            .body(&format!("Launching {}", app.name))
            .icon("totray")
            .show();
    }

    launch_captured(app, state.clone());

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

        if delay > 500 {
            thread::sleep(Duration::from_millis(200));
        }
    }

    match app.action {
        Action::Close => hypr::close_window(&app.name),
        Action::Close2 => {
            let mut found = false;
            for _ in 0..40 {
                if hypr::get_window_count(&app.name) > 0 {
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

        if let Ok(mut c) = child {
            let stdout = c.stdout.take().unwrap();
            let stderr = c.stderr.take().unwrap();

            let st_out = state.clone();
            let name_out = name.clone();
            thread::spawn(move || {
                let reader = BufReader::new(stdout);
                for l in reader.lines().map_while(Result::ok) {
                    let mut s = st_out.lock().unwrap();
                    s.logs.entry(name_out.clone()).or_default().push(l);
                    if s.logs.get(&name_out).unwrap().len() > 500 {
                        s.logs.get_mut(&name_out).unwrap().remove(0);
                    }
                }
            });

            let st_err = state.clone();
            let name_err = name.clone();
            thread::spawn(move || {
                let reader = BufReader::new(stderr);
                for l in reader.lines().map_while(Result::ok) {
                    let mut s = st_err.lock().unwrap();
                    s.logs
                        .entry(name_err.clone())
                        .or_default()
                        .push(format!("[ERR] {}", l));
                    if s.logs.get(&name_err).unwrap().len() > 500 {
                        s.logs.get_mut(&name_err).unwrap().remove(0);
                    }
                }
            });

            let _ = c.wait();
        }
    });
}

pub fn autostart(add: bool) {
    if let Some(config_dir) = dirs::config_dir() {
        let hypr_conf = config_dir.join("hypr/hyprland.conf");
        if let Ok(content) = std::fs::read_to_string(&hypr_conf) {
            let mut lines: Vec<String> = content.lines().map(|s| s.to_string()).collect();
            lines.retain(|l| !l.contains("totray") && !l.contains("# ToTray:"));
            while lines.last().map(|l| l.trim().is_empty()).unwrap_or(false) {
                lines.pop();
            }
            if add {
                let mut bin_path = dirs::home_dir()
                    .map(|h| h.join(".local/bin/totray"))
                    .unwrap_or_else(|| std::path::PathBuf::from("totray"));
                if !bin_path.exists()
                    && let Ok(current) = std::env::current_exe()
                {
                    bin_path = current;
                }
                lines.push("".to_string());
                lines.push("# ToTray: Autorun manager for Hyprland".to_string());
                lines.push(format!("exec-once = {}", bin_path.display()));
            }
            let _ = std::fs::write(&hypr_conf, lines.join("\n") + "\n");
        }
    }
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

        if let Ok(current_exe) = std::env::current_exe() {
            if target_bin.exists() {
                let _ = std::fs::remove_file(&target_bin);
            }

            match std::fs::copy(&current_exe, &target_bin) {
                Ok(_) => {
                    #[cfg(unix)]
                    {
                        use std::os::unix::fs::PermissionsExt;
                        let _ = std::fs::set_permissions(
                            &target_bin,
                            std::fs::Permissions::from_mode(0o755),
                        );
                    }
                    println!("Successfully installed binary to {}", target_bin.display());
                }
                Err(e) => {
                    eprintln!(
                        "Failed to copy binary from {} to {}: {}",
                        current_exe.display(),
                        target_bin.display(),
                        e
                    );
                    if let Ok(bytes) = std::fs::read(&current_exe) {
                        if let Err(e2) = std::fs::write(&target_bin, bytes) {
                            eprintln!("Fallback write also failed: {}", e2);
                        } else {
                            #[cfg(unix)]
                            {
                                use std::os::unix::fs::PermissionsExt;
                                let _ = std::fs::set_permissions(
                                    &target_bin,
                                    std::fs::Permissions::from_mode(0o755),
                                );
                            }
                            println!("Fallback installation successful.");
                        }
                    }
                }
            }
        } else {
            eprintln!("Could not determine current executable path");
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
             Exec={} --gui\n\
             Icon=totray\n\
             Terminal=false\n\
             Type=Application\n\
             Categories=Utility;System;\n\
             StartupNotify=false\n\
             Actions=Settings;\n\n\
             [Desktop Action Settings]\n\
             Name=Settings\n\
             Exec={} --gui\n",
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
