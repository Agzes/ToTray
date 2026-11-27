mod backend;
mod hypr;
mod state;
mod tray;
mod ui;

use clap::Parser;
use state::SharedState;
use std::io::{Read, Write};
use std::os::unix::net::{UnixListener, UnixStream};
use std::path::PathBuf;
use std::process::ExitCode;
use std::sync::{Arc, Mutex};
use std::time::Duration;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    #[arg(short, long, help = "Start worker (wait for Hyprland and run rules)")]
    worker: bool,

    #[arg(short, long, help = "Start minimized in the system tray")]
    tray: bool,

    #[arg(long, help = "Check if ToTray is correctly installed")]
    hello: bool,

    #[arg(long, help = "Add a new rule via CLI")]
    add: bool,

    #[arg(long, help = "Window class name for the new rule")]
    name: Option<String>,

    #[arg(long, help = "Execution command for the new rule")]
    exec: Option<String>,

    #[arg(
        long,
        help = "Action for the new rule (close, close2, closetimes, workspace, special, tray, float, fullscreen, launch)"
    )]
    action: Option<String>,

    #[arg(
        long,
        help = "Repeat count (used for closetimes, tray, workspace, special)"
    )]
    count: Option<u32>,

    #[arg(long, help = "Workspace number (required if action is 'workspace')")]
    workspace: Option<u32>,

    #[arg(long, help = "Special workspace name (used when action is 'special')")]
    special: Option<String>,

    #[arg(long, help = "Print version information in JSON format")]
    version_json: bool,

    #[arg(long, help = "Print active rules in JSON format")]
    config_json: bool,

    #[arg(long, help = "Install desktop entry, icon, and binary to ~/.local/bin")]
    install: bool,

    #[arg(long, help = "Uninstall desktop entry, icons, and binary")]
    uninstall: bool,

    #[arg(long, help = "Purge all config files, service, and desktop entries")]
    purge: bool,
}

fn socket_path() -> PathBuf {
    dirs::runtime_dir()
        .unwrap_or_else(std::env::temp_dir)
        .join("totray.sock")
}

fn notify_existing_instance(show_window: bool) -> bool {
    let path = socket_path();
    let Ok(mut stream) = UnixStream::connect(&path) else {
        return false;
    };
    if show_window {
        let _ = stream.write_all(b"show\n");
    }
    true
}

fn spawn_ipc_listener(show_requests: Arc<Mutex<bool>>) {
    let path = socket_path();
    let _ = std::fs::remove_file(&path);

    let Ok(listener) = UnixListener::bind(&path) else {
        eprintln!("[ToTray] Could not bind IPC socket at {}", path.display());
        return;
    };

    std::thread::spawn(move || {
        for stream in listener.incoming().flatten() {
            let mut s = stream;
            let mut buf = [0u8; 32];
            if let Ok(n) = s.read(&mut buf) {
                let cmd = String::from_utf8_lossy(&buf[..n]);
                if cmd.trim() == "show" {
                    *show_requests.lock().unwrap() = true;
                }
            }
        }
    });
}

fn main() -> ExitCode {
    let args = Args::parse();

    if args.hello {
        println!("ToTray is correctly installed and working!");
        return ExitCode::SUCCESS;
    }

    if args.version_json {
        let v = serde_json::json!({
            "version": ui::CURRENT_VERSION,
            "name": "ToTray",
            "author": "agzes"
        });
        println!("{}", serde_json::to_string_pretty(&v).unwrap());
        return ExitCode::SUCCESS;
    }

    if args.install {
        if let Err(e) = backend::install_desktop_entry() {
            eprintln!("Error during installation: {e}");
            return ExitCode::from(1);
        }
        println!("Successfully installed desktop entry, icon, and binary.");
        return ExitCode::SUCCESS;
    }

    if args.uninstall {
        if let Err(e) = backend::uninstall_desktop_entry() {
            eprintln!("Error during uninstallation: {e}");
            return ExitCode::from(1);
        }
        println!("Successfully removed desktop entry, icons, and binary.");
        return ExitCode::SUCCESS;
    }

    if args.purge {
        if backend::uninstall() {
            println!("Successfully purged all configuration files, service, and desktop entries.");
            return ExitCode::SUCCESS;
        }
        return ExitCode::from(1);
    }

    let state: SharedState = Arc::new(Mutex::new(state::AppState::load()));

    if args.config_json {
        let s = state.lock().unwrap();
        println!("{}", serde_json::to_string_pretty(&s.apps).unwrap());
        return ExitCode::SUCCESS;
    }

    if args.add {
        let Some(name) = args.name else {
            eprintln!("Missing --name");
            return ExitCode::from(1);
        };
        let Some(exec) = args.exec else {
            eprintln!("Missing --exec");
            return ExitCode::from(1);
        };
        let Some(action_str) = args.action else {
            eprintln!("Missing --action (close, close2, workspace, tray)");
            return ExitCode::from(1);
        };

        let count = args.count.unwrap_or(1).max(1);
        let action = match action_str.to_lowercase().as_str() {
            "close" => state::Action::Close,
            "close2" => state::Action::Close2,
            "closetimes" => state::Action::CloseTimes(args.count.unwrap_or(2).max(1)),
            "workspace" => {
                let Some(ws) = args.workspace else {
                    eprintln!("Missing --workspace #");
                    return ExitCode::from(1);
                };
                if count > 1 {
                    state::Action::WorkspaceTimes(ws, count)
                } else {
                    state::Action::Workspace(ws)
                }
            }
            "special" => {
                let name = args.special.unwrap_or_else(|| "shelf".to_string());
                if count > 1 {
                    state::Action::SpecialWorkspaceTimes(name, count)
                } else {
                    state::Action::SpecialWorkspace(name)
                }
            }
            "tray" => {
                if count > 1 {
                    state::Action::HideToTrayTimes(count)
                } else {
                    state::Action::HideToTray
                }
            }
            "float" => state::Action::Float,
            "fullscreen" => state::Action::Fullscreen,
            "launch" => state::Action::LaunchOnly,
            _ => {
                eprintln!(
                    "Invalid action. Use close, close2, closetimes, workspace, special, tray, float, fullscreen, or launch."
                );
                return ExitCode::from(1);
            }
        };

        {
            let mut s = state.lock().unwrap();
            s.apps.push(state::AppRule {
                name: name.clone(),
                exec,
                action,
            });
            s.save();
        }
        println!("Added rule for {name}");
        return ExitCode::SUCCESS;
    }

    let start_hidden = args.tray || (args.worker && state.lock().unwrap().silent_mode);

    if notify_existing_instance(!start_hidden) {
        return ExitCode::SUCCESS;
    }

    let show_requests = Arc::new(Mutex::new(false));
    spawn_ipc_listener(show_requests.clone());

    if args.worker {
        let auto_start = { state.lock().unwrap().auto_start };
        backend::migrate_legacy_autostart(auto_start);

        if !hypr::wait_for_session(Duration::from_secs(120)) {
            eprintln!("[ToTray] Hyprland is not ready, continuing anyway...");
        }
        println!("[ToTray] Starting rules engine...");
        backend::start_backend(state.clone());
    }

    let result = ui::run(state, show_requests, start_hidden);
    let _ = std::fs::remove_file(socket_path());

    match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("UI error: {error}");
            ExitCode::from(1)
        }
    }
}
