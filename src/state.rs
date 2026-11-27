use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

pub const APP_ID: &str = "dev.agzes.totray";
pub const APP_SLUG: &str = "totray";
pub const APP_TITLE: &str = "ToTray";

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub enum Action {
    Close,
    Close2,
    CloseTimes(u32),
    Workspace(u32),
    WorkspaceTimes(u32, u32),
    SpecialWorkspace(String),
    SpecialWorkspaceTimes(String, u32),
    HideToTray,
    HideToTrayTimes(u32),
    Float,
    Fullscreen,
    LaunchOnly,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct AppRule {
    pub name: String,
    pub exec: String,
    pub action: Action,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct AppState {
    pub apps: Vec<AppRule>,
    pub launch_delay: u64,
    pub notifications: bool,
    pub auto_start: bool,
    pub multi_launch: bool,
    pub silent_mode: bool,

    pub theme_sync: bool,
    pub theme_dark: bool,
    pub system_colors: bool,

    pub last_run_version: Option<String>,
    pub shown_warning: bool,
    pub desktop_installed: bool,

    #[serde(skip)]
    pub hidden_apps: Vec<String>,
    #[serde(skip)]
    pub logs: HashMap<String, Vec<String>>,
    #[serde(skip)]
    pub running_rules: usize,
    #[serde(skip)]
    pub error_message: Option<String>,
}

impl Default for AppState {
    fn default() -> Self {
        Self {
            apps: vec![],
            launch_delay: 0,
            notifications: true,
            auto_start: false,
            multi_launch: true,
            silent_mode: true,
            theme_sync: false,
            theme_dark: true,
            system_colors: false,
            last_run_version: None,
            shown_warning: false,
            desktop_installed: false,
            hidden_apps: vec![],
            logs: HashMap::new(),
            running_rules: 0,
            error_message: None,
        }
    }
}

pub type SharedState = Arc<Mutex<AppState>>;

pub fn push_log(state: &SharedState, name: &str, line: impl Into<String>) {
    let mut s = state.lock().unwrap();
    let entry = s.logs.entry(name.to_string()).or_default();
    entry.push(line.into());
    if entry.len() > 500 {
        let excess = entry.len() - 500;
        entry.drain(..excess);
    }
}

impl AppState {
    fn get_config_path() -> Option<PathBuf> {
        let mut path = dirs::config_dir()?;
        path.push(APP_SLUG);
        if !path.exists() {
            let _ = fs::create_dir_all(&path);
        }
        path.push("config.json");
        Some(path)
    }

    pub fn load() -> Self {
        if let Some(path) = Self::get_config_path()
            && let Ok(data) = fs::read_to_string(path)
            && let Ok(state) = serde_json::from_str::<AppState>(&data)
        {
            return state;
        }
        Self::default()
    }

    pub fn save(&self) {
        if let Some(path) = Self::get_config_path()
            && let Ok(data) = serde_json::to_string_pretty(self)
        {
            let _ = fs::write(path, data);
        }
    }

    pub fn export_config(&self, path: PathBuf) -> Result<(), String> {
        let data = serde_json::to_string_pretty(self)
            .map_err(|e| format!("Failed to serialize config: {e}"))?;
        fs::write(path, data).map_err(|e| format!("Failed to write file: {e}"))
    }

    pub fn import_config(&mut self, path: PathBuf) -> Result<(), String> {
        let data = fs::read_to_string(path).map_err(|e| format!("Failed to read file: {e}"))?;
        let mut new_state: AppState =
            serde_json::from_str(&data).map_err(|e| format!("Failed to parse config: {e}"))?;

        new_state.hidden_apps = vec![];
        new_state.logs = HashMap::new();
        new_state.running_rules = 0;
        new_state.error_message = None;
        *self = new_state;
        self.save();
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_state() {
        let state = AppState::default();
        assert_eq!(state.apps.len(), 0);
        assert_eq!(state.launch_delay, 0);
        assert!(state.notifications);
        assert!(!state.auto_start);
        assert!(state.multi_launch);
        assert!(state.silent_mode);
        assert!(!state.theme_sync);
    }

    #[test]
    fn test_serialization() {
        let mut state = AppState::default();
        state.apps.push(AppRule {
            name: "test".to_string(),
            exec: "test-exec".to_string(),
            action: Action::HideToTray,
        });

        let json = serde_json::to_string(&state).unwrap();
        let decoded: AppState = serde_json::from_str(&json).unwrap();

        assert_eq!(decoded.apps.len(), 1);
        assert_eq!(decoded.apps[0].name, "test");
        assert_eq!(decoded.apps[0].action, Action::HideToTray);
    }

    #[test]
    fn test_legacy_config_deserialization() {
        let legacy = r#"{
            "apps": [],
            "launch_delay": 500,
            "notifications": false,
            "auto_start": true,
            "multi_launch": false,
            "silent_mode": true,
            "last_run_version": "0.2.0",
            "shown_warning": true,
            "desktop_installed": true
        }"#;
        let decoded: AppState = serde_json::from_str(legacy).unwrap();
        assert_eq!(decoded.launch_delay, 500);
        assert!(decoded.auto_start);
        assert!(!decoded.theme_sync);
    }

    #[test]
    fn test_action_serialization() {
        let actions = vec![
            Action::Close,
            Action::Close2,
            Action::CloseTimes(3),
            Action::Workspace(5),
            Action::WorkspaceTimes(5, 2),
            Action::SpecialWorkspace("shelf".to_string()),
            Action::SpecialWorkspaceTimes("shelf".to_string(), 2),
            Action::HideToTray,
            Action::HideToTrayTimes(2),
            Action::Float,
            Action::Fullscreen,
            Action::LaunchOnly,
        ];

        for action in actions {
            let json = serde_json::to_string(&action).unwrap();
            let decoded: Action = serde_json::from_str(&json).unwrap();
            assert_eq!(action, decoded);
        }
    }
}
