mod backend;
mod hypr;
mod state;
mod ui;

use clap::Parser;
use gtk::Application;
use gtk::prelude::*;
use ksni::{
    Icon, ToolTip, Tray, TrayService,
    menu::{MenuItem, StandardItem},
};
use state::SharedState;
use std::sync::{Arc, Mutex};

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    #[arg(short, long, help = "Launch the settings GUI")]
    gui: bool,

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
        help = "Action for the new rule (close, close2, workspace, tray)"
    )]
    action: Option<String>,

    #[arg(long, help = "Workspace number (required if action is 'workspace')")]
    workspace: Option<u32>,

    #[arg(long, help = "Print version information in JSON format")]
    version_json: bool,

    #[arg(long, help = "Print active rules in JSON format")]
    config_json: bool,
}

struct ToTrayIcon {
    state: SharedState,
}

impl Tray for ToTrayIcon {
    fn icon_name(&self) -> String {
        "totray".into()
    }
    fn icon_pixmap(&self) -> Vec<Icon> {
        let logo_data = include_bytes!("../assets/logo.png");
        if let Ok(img) = image::load_from_memory(logo_data) {
            let rgba = img.to_rgba8();
            let (width, height) = rgba.dimensions();
            let mut argb = Vec::with_capacity((width * height * 4) as usize);
            for chunk in rgba.chunks_exact(4) {
                argb.push(chunk[3]);
                argb.push(chunk[0]);
                argb.push(chunk[1]);
                argb.push(chunk[2]);
            }
            return vec![Icon {
                width: width as i32,
                height: height as i32,
                data: argb,
            }];
        }
        vec![]
    }
    fn id(&self) -> String {
        "totray".into()
    }
    fn title(&self) -> String {
        "ToTray".into()
    }
    fn tool_tip(&self) -> ToolTip {
        let description = if let Ok(s) = self.state.lock() {
            format!(
                "Managing {} active rules • {} apps hidden",
                s.apps.len(),
                s.hidden_apps.len()
            )
        } else {
            "Auto-run & Tray Manager".into()
        };
        ToolTip {
            title: "ToTray".into(),
            description,
            ..Default::default()
        }
    }
    fn menu(&self) -> Vec<MenuItem<Self>> {
        let mut menu = vec![
            MenuItem::Standard(StandardItem {
                label: "Settings".into(),
                activate: Box::new(|_| {
                    let exe = std::env::current_exe().unwrap_or_else(|_| "totray".into());
                    let _ = std::process::Command::new(exe).arg("--gui").spawn();
                }),
                ..Default::default()
            }),
            MenuItem::Standard(StandardItem {
                label: "Quit".into(),
                activate: Box::new(|_| std::process::exit(0)),
                ..Default::default()
            }),
            MenuItem::Separator,
        ];

        if let Ok(s) = self.state.lock() {
            for app in &s.hidden_apps {
                let app_name = app.clone();
                let state_c = self.state.clone();
                menu.push(MenuItem::Standard(StandardItem {
                    label: format!("Show {}", app_name),
                    activate: Box::new(move |_| {
                        hypr::show_from_special(&app_name);
                        if let Ok(mut s) = state_c.lock() {
                            s.hidden_apps.retain(|a| a != &app_name);
                        }
                    }),
                    ..Default::default()
                }));
            }
        }

        menu
    }
}

fn main() -> glib::ExitCode {
    let args = Args::parse();

    if args.hello {
        println!("ToTray is correctly installed and working!");
        return glib::ExitCode::from(0);
    }

    if args.version_json {
        let v = serde_json::json!({
            "version": ui::CURRENT_VERSION,
            "name": "ToTray",
            "author": "agzes"
        });
        println!("{}", serde_json::to_string_pretty(&v).unwrap());
        return glib::ExitCode::from(0);
    }

    let state: SharedState = Arc::new(Mutex::new(state::AppState::load()));

    if args.config_json {
        let s = state.lock().unwrap();
        println!("{}", serde_json::to_string_pretty(&s.apps).unwrap());
        return glib::ExitCode::from(0);
    }

    if args.add {
        let name = args.name.expect("Missing --name");
        let exec = args.exec.expect("Missing --exec");
        let action_str = args
            .action
            .expect("Missing --action (close, close2, workspace, tray)");

        let action = match action_str.to_lowercase().as_str() {
            "close" => state::Action::Close,
            "close2" => state::Action::Close2,
            "workspace" => state::Action::Workspace(args.workspace.expect("Missing --workspace #")),
            "tray" => state::Action::HideToTray,
            _ => panic!("Invalid action. Use close, close2, workspace, or tray."),
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
        println!("Added rule for {}", name);
        return glib::ExitCode::from(0);
    }

    {
        let s = state.lock().unwrap();
        backend::autostart(s.auto_start);
    }

    let silent = { state.lock().unwrap().silent_mode };

    if !args.gui {
        println!("ToTray backend starting...");
        backend::start_backend(state.clone());
        let tray = ToTrayIcon {
            state: state.clone(),
        };
        let service = TrayService::new(tray);
        service.spawn();

        if silent {
            let state_ui = state.clone();
            glib::timeout_add_local_once(std::time::Duration::from_millis(100), move || {
                let app = Application::builder().application_id(state::APP_ID).build();
                app.connect_activate(move |obj| {
                    ui::build_ui(obj, state_ui.clone());
                });
                app.run_with_args::<&str>(&[]);
            });
        }

        println!("Tray icon active. Press Ctrl+C to stop.");
        glib::MainLoop::new(None, false).run();
        return glib::ExitCode::from(0);
    }

    let app = Application::builder().application_id(state::APP_ID).build();
    let state_ui = state.clone();

    app.connect_activate(move |obj| {
        if let Some(window) = obj.windows().first() {
            window.present();
            return;
        }

        ui::build_ui(obj, state_ui.clone());

        let tray = ToTrayIcon {
            state: state_ui.clone(),
        };
        let service = TrayService::new(tray);
        service.spawn();
    });

    app.run_with_args::<&str>(&[])
}
