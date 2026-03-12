use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

pub const APP_ID: &str = "dev.agzes.totray";

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub enum Action {
    Close,
    Close2,
    Workspace(u32),
    HideToTray,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct AppRule {
    pub name: String,
    pub exec: String,
    pub action: Action,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AppState {
    pub apps: Vec<AppRule>,
    pub launch_delay: u64,
    pub notifications: bool,
    pub auto_start: bool,
    pub silent_mode: bool,

    pub last_run_version: Option<String>,
    pub shown_warning: bool,
    pub desktop_installed: bool,

    #[serde(skip)]
    pub hidden_apps: Vec<String>,
    #[serde(skip)]
    pub logs: std::collections::HashMap<String, Vec<String>>,
}

impl Default for AppState {
    fn default() -> Self {
        Self {
            apps: vec![],
            launch_delay: 1500,
            notifications: true,
            auto_start: false,
            silent_mode: false,
            last_run_version: None,
            shown_warning: false,
            desktop_installed: false,
            hidden_apps: vec![],
            logs: std::collections::HashMap::new(),
        }
    }
}

pub type SharedState = Arc<Mutex<AppState>>;

impl AppState {
    fn get_config_path() -> Option<PathBuf> {
        let mut path = dirs::config_dir()?;
        path.push("totray");
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
        assert_eq!(state.launch_delay, 1500);
        assert!(state.notifications);
        assert!(!state.auto_start);
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
    fn test_action_serialization() {
        let actions = vec![
            Action::Close,
            Action::Close2,
            Action::Workspace(5),
            Action::HideToTray,
        ];

        for action in actions {
            let json = serde_json::to_string(&action).unwrap();
            let decoded: Action = serde_json::from_str(&json).unwrap();
            assert_eq!(action, decoded);
        }
    }
}
