use crate::state::{APP_ID, APP_TITLE, SharedState};
use image::GenericImageView;
use ksni::blocking::{Handle, TrayMethods};
use std::sync::{Arc, LazyLock, Mutex};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TrayCommand {
    Show,
    ShowApp(String),
    Quit,
}

fn load_icon(data: &[u8]) -> Vec<ksni::Icon> {
    if let Ok(img) = image::load_from_memory(data) {
        let img = img.resize(32, 32, image::imageops::FilterType::Lanczos3);
        let (w, h) = img.dimensions();
        let rgba = img.to_rgba8();
        let mut pixels = Vec::with_capacity((w * h * 4) as usize);
        for pixel in rgba.pixels() {
            pixels.push(pixel[3]);
            pixels.push(pixel[0]);
            pixels.push(pixel[1]);
            pixels.push(pixel[2]);
        }
        vec![ksni::Icon {
            width: w as i32,
            height: h as i32,
            data: pixels,
        }]
    } else {
        vec![]
    }
}

static TRAY_ICON: LazyLock<Vec<ksni::Icon>> =
    LazyLock::new(|| load_icon(include_bytes!("../assets/logo.png")));

struct ToTrayIcon {
    queue: Arc<Mutex<Vec<TrayCommand>>>,
    state: SharedState,
}

impl ToTrayIcon {
    fn item(&self, label: &str, command: TrayCommand) -> ksni::MenuItem<Self> {
        use ksni::menu::StandardItem;
        let queue = self.queue.clone();
        ksni::MenuItem::Standard(StandardItem {
            label: label.into(),
            activate: Box::new(move |_| {
                queue.lock().unwrap().push(command.clone());
            }),
            ..Default::default()
        })
    }
}

impl ksni::Tray for ToTrayIcon {
    fn icon_name(&self) -> String {
        "totray".into()
    }

    fn icon_pixmap(&self) -> Vec<ksni::Icon> {
        TRAY_ICON.clone()
    }

    fn id(&self) -> String {
        APP_ID.into()
    }

    fn title(&self) -> String {
        APP_TITLE.into()
    }

    fn tool_tip(&self) -> ksni::ToolTip {
        let description = if let Ok(s) = self.state.lock() {
            format!(
                "Managing {} active rules • {} apps hidden",
                s.apps.len(),
                s.hidden_apps.len()
            )
        } else {
            "Auto-run & Tray Manager".into()
        };
        ksni::ToolTip {
            title: APP_TITLE.into(),
            description,
            ..Default::default()
        }
    }

    fn menu(&self) -> Vec<ksni::MenuItem<Self>> {
        let mut menu = vec![
            self.item("Settings", TrayCommand::Show),
            self.item("Quit", TrayCommand::Quit),
        ];

        if let Ok(s) = self.state.lock()
            && !s.hidden_apps.is_empty()
        {
            menu.push(ksni::MenuItem::Separator);
            for app in &s.hidden_apps {
                menu.push(self.item(&format!("Show {app}"), TrayCommand::ShowApp(app.clone())));
            }
        }

        menu
    }
}

pub struct TrayBridge {
    handle: Handle<ToTrayIcon>,
    queue: Arc<Mutex<Vec<TrayCommand>>>,
    last_snapshot: Option<(usize, Vec<String>)>,
}

impl TrayBridge {
    pub fn spawn(state: SharedState) -> Option<Self> {
        let queue = Arc::new(Mutex::new(Vec::new()));
        let tray = ToTrayIcon {
            queue: queue.clone(),
            state,
        };
        let handle = tray.spawn().ok()?;
        Some(Self {
            handle,
            queue,
            last_snapshot: None,
        })
    }

    pub fn drain(&self) -> Vec<TrayCommand> {
        let mut queue = self.queue.lock().unwrap();
        std::mem::take(&mut *queue)
    }

    pub fn refresh(&mut self, rules_count: usize, hidden_apps: &[String]) {
        let next = (rules_count, hidden_apps.to_vec());
        if self.last_snapshot.as_ref() == Some(&next) {
            return;
        }
        self.last_snapshot = Some(next);
        let _ = self.handle.update(|_| {});
    }
}
