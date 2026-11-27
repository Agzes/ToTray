use crate::backend::DesktopIntegrationStatus;
use crate::hypr;
use crate::state::{APP_TITLE, Action, AppRule, AppState, SharedState};
use crate::tray::{TrayBridge, TrayCommand};
use iced::alignment::{Horizontal, Vertical};
use iced::theme::Mode as ThemeMode;
use iced::widget::tooltip::Position as TooltipPosition;
use iced::widget::{
    Space, button, column, container, pick_list, row, scrollable, stack, svg, text, text_input,
    tooltip,
};
use iced::{
    Background, Border, Color, Element, Fill, Font, Length, Padding, Shadow, Size, Subscription,
    Task, Theme, font, time, window,
};
use std::fmt;
use std::path::PathBuf;
use std::process::Command;
use std::sync::{Arc, Mutex};
use std::time::Duration;

pub const CURRENT_VERSION: &str = env!("CARGO_PKG_VERSION");
const REPOSITORY_URL: &str = "https://github.com/agzes/ToTray";
const RELEASES_URL: &str = "https://github.com/agzes/ToTray/releases";
const TICK: Duration = Duration::from_millis(250);
const FRAME: Duration = Duration::from_millis(16);

const BOLD: Font = Font {
    weight: font::Weight::Bold,
    ..Font::DEFAULT
};

const SEMIBOLD: Font = Font {
    weight: font::Weight::Semibold,
    ..Font::DEFAULT
};

const MONOSPACE: Font = Font::MONOSPACE;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Icon {
    Settings,
    Repository,
    Info,
    Check,
    Warning,
    Layers,
    Autostart,
    Notifications,
    Hide,
    Multi,
    Interval,
    Action,
    Terminal,
    Trash,
    Edit,
    Plus,
    Palette,
    Moon,
    Close,
    WindowClose,
    Float,
    Fullscreen,
    File,
}

impl Icon {
    fn bytes(self) -> &'static [u8] {
        match self {
            Self::Settings => include_bytes!("../assets/icons/settings.svg"),
            Self::Repository => include_bytes!("../assets/icons/repository.svg"),
            Self::Info => include_bytes!("../assets/icons/info.svg"),
            Self::Check => include_bytes!("../assets/icons/check.svg"),
            Self::Warning => include_bytes!("../assets/icons/warning.svg"),
            Self::Layers => include_bytes!("../assets/icons/layers.svg"),
            Self::Autostart => include_bytes!("../assets/icons/autostart.svg"),
            Self::Notifications => include_bytes!("../assets/icons/notifications.svg"),
            Self::Hide => include_bytes!("../assets/icons/hide.svg"),
            Self::Multi => include_bytes!("../assets/icons/multi.svg"),
            Self::Interval => include_bytes!("../assets/icons/interval.svg"),
            Self::Action => include_bytes!("../assets/icons/action.svg"),
            Self::Terminal => include_bytes!("../assets/icons/terminal.svg"),
            Self::Trash => include_bytes!("../assets/icons/trash.svg"),
            Self::Edit => include_bytes!("../assets/icons/edit.svg"),
            Self::Plus => include_bytes!("../assets/icons/plus.svg"),
            Self::Palette => include_bytes!("../assets/icons/palette.svg"),
            Self::Moon => include_bytes!("../assets/icons/moon.svg"),
            Self::Close => include_bytes!("../assets/icons/close.svg"),
            Self::WindowClose => include_bytes!("../assets/icons/window-close.svg"),
            Self::Float => include_bytes!("../assets/icons/float.svg"),
            Self::Fullscreen => include_bytes!("../assets/icons/fullscreen.svg"),
            Self::File => include_bytes!("../assets/icons/file.svg"),
        }
    }
}

fn icon<'a>(which: Icon, size: f32, color: Color) -> Element<'a, Message> {
    let handle = svg::Handle::from_memory(which.bytes());
    svg(handle)
        .width(Length::Fixed(size))
        .height(Length::Fixed(size))
        .style(move |_theme, _status| svg::Style { color: Some(color) })
        .into()
}

#[derive(Clone, Copy)]
struct Palette {
    fg: Color,
    muted: Color,
    faint: Color,
    accent: Color,
    on_accent: Color,
    success: Color,
    warning: Color,
    danger: Color,
    surface: Color,
    surface_hover: Color,
    card: Color,
    strip: Color,
    hairline: Color,
    border: Color,
}

impl Palette {
    fn of(theme: &Theme) -> Self {
        let base = theme.palette();
        let extended = theme.extended_palette();
        let fg = base.text;
        let card = if extended.is_dark {
            Color::from_rgb8(0x19, 0x19, 0x1B)
        } else {
            Color::WHITE
        };
        Self {
            fg,
            muted: alpha(fg, 0.62),
            faint: alpha(fg, 0.38),
            accent: base.primary,
            on_accent: extended.primary.base.text,
            success: base.success,
            warning: base.warning,
            danger: base.danger,
            surface: alpha(fg, 0.05),
            surface_hover: alpha(fg, 0.11),
            card,
            strip: mix(base.background, card, 0.5),
            hairline: alpha(fg, 0.08),
            border: alpha(fg, 0.14),
        }
    }
}

fn select_style(palette: Palette) -> impl Fn(&Theme, pick_list::Status) -> pick_list::Style {
    move |_theme, status| pick_list::Style {
        text_color: palette.fg,
        placeholder_color: palette.muted,
        handle_color: palette.muted,
        background: Background::Color(match status {
            pick_list::Status::Hovered => palette.surface_hover,
            _ => palette.surface,
        }),
        border: Border {
            color: palette.border,
            width: 1.0,
            radius: 9.0.into(),
        },
    }
}

fn input_style(palette: Palette) -> impl Fn(&Theme, text_input::Status) -> text_input::Style {
    move |_theme, _status| text_input::Style {
        background: Background::Color(palette.surface),
        border: Border {
            color: palette.border,
            width: 1.0,
            radius: 9.0.into(),
        },
        icon: palette.muted,
        placeholder: palette.faint,
        value: palette.fg,
        selection: palette.accent,
    }
}

fn alpha(color: Color, a: f32) -> Color {
    Color { a, ..color }
}

const DEFAULT_ACCENT: Color = Color::from_rgb8(0xC0, 0x8E, 0x79);
const DARK_BG: Color = Color::from_rgb8(0x10, 0x10, 0x10);
const SUCCESS_DARK: Color = Color::from_rgb8(0x4A, 0xDE, 0x80);
const SUCCESS_LIGHT: Color = Color::from_rgb8(0x15, 0x80, 0x3D);
const DANGER_DARK: Color = Color::from_rgb8(0xF8, 0x71, 0x71);
const DANGER_LIGHT: Color = Color::from_rgb8(0xDC, 0x26, 0x26);

fn custom_theme(dark: bool, accent: Option<Color>, system: Option<SystemColors>) -> Theme {
    let mut palette = if dark {
        iced::theme::Palette {
            background: DARK_BG,
            text: Color::from_rgb8(0xE7, 0xE9, 0xEF),
            primary: accent.unwrap_or(DEFAULT_ACCENT),
            success: SUCCESS_DARK,
            warning: Color::from_rgb8(0xFB, 0xBF, 0x24),
            danger: DANGER_DARK,
        }
    } else {
        iced::theme::Palette {
            background: Color::from_rgb8(0xF4, 0xF5, 0xF8),
            text: Color::from_rgb8(0x1A, 0x1C, 0x21),
            primary: accent.unwrap_or(DEFAULT_ACCENT),
            success: SUCCESS_LIGHT,
            warning: Color::from_rgb8(0xB4, 0x5D, 0x09),
            danger: DANGER_LIGHT,
        }
    };

    if let Some(system) = system {
        palette.background = system.background;
        palette.text = system.text;
        palette.success = system.success;
        palette.danger = system.danger;
    }

    Theme::custom(if dark { "ToTray Dark" } else { "ToTray Light" }, palette)
}

fn mix(a: Color, b: Color, t: f32) -> Color {
    Color {
        r: a.r + (b.r - a.r) * t,
        g: a.g + (b.g - a.g) * t,
        b: a.b + (b.b - a.b) * t,
        a: a.a + (b.a - a.a) * t,
    }
}

const ACTION_CHOICES: [ActionChoice; 8] = [
    ActionChoice::HideToTray,
    ActionChoice::Close,
    ActionChoice::CloseTimes,
    ActionChoice::Workspace,
    ActionChoice::SpecialWorkspace,
    ActionChoice::Float,
    ActionChoice::Fullscreen,
    ActionChoice::LaunchOnly,
];

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum ActionChoice {
    HideToTray,
    Close,
    CloseTimes,
    Workspace,
    SpecialWorkspace,
    Float,
    Fullscreen,
    LaunchOnly,
}

impl ActionChoice {
    fn label(self) -> &'static str {
        match self {
            Self::HideToTray => "To Tray",
            Self::Close => "Close",
            Self::CloseTimes => "Close X times",
            Self::Workspace => "Workspace",
            Self::SpecialWorkspace => "Special WS",
            Self::Float => "Float",
            Self::Fullscreen => "Fullscreen",
            Self::LaunchOnly => "Launch Only",
        }
    }
}

impl fmt::Display for ActionChoice {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.label())
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Flag {
    AutoStart,
    Notifications,
    SilentMode,
    MultiLaunch,
    ThemeSync,
    ThemeDark,
    SystemColors,
}

const FLAGS: [Flag; Flag::COUNT] = [
    Flag::AutoStart,
    Flag::Notifications,
    Flag::SilentMode,
    Flag::MultiLaunch,
    Flag::ThemeSync,
    Flag::ThemeDark,
    Flag::SystemColors,
];

impl Flag {
    const COUNT: usize = 7;

    fn index(self) -> usize {
        self as usize
    }

    fn apply(self, state: &mut AppState, value: bool) {
        match self {
            Self::AutoStart => state.auto_start = value,
            Self::Notifications => state.notifications = value,
            Self::SilentMode => state.silent_mode = value,
            Self::MultiLaunch => state.multi_launch = value,
            Self::ThemeSync => state.theme_sync = value,
            Self::ThemeDark => state.theme_dark = value,
            Self::SystemColors => state.system_colors = value,
        }
    }
}

fn flag_value(state: &AppState, flag: Flag) -> bool {
    match flag {
        Flag::AutoStart => state.auto_start,
        Flag::Notifications => state.notifications,
        Flag::SilentMode => state.silent_mode,
        Flag::MultiLaunch => state.multi_launch,
        Flag::ThemeSync => state.theme_sync,
        Flag::ThemeDark => state.theme_dark,
        Flag::SystemColors => state.system_colors,
    }
}

#[derive(Clone, PartialEq, Eq)]
enum Page {
    Rules,
    Settings,
    Logs(String),
}

#[derive(Clone, Debug)]
enum Message {
    Tick,
    Frame,
    ThemeModeChanged(ThemeMode),
    WindowOpened(window::Id),
    CloseRequested(window::Id),
    DismissWelcome,
    ToggleSettings,
    BackToRules,
    OpenUrl(&'static str),
    RunAllRules,
    RunRule(usize),
    StartEditRule(usize),
    CancelEditRule,
    DeleteRule(usize),
    OpenLogs(String),
    ClearLogs(String),
    ToggleAddRule,
    SelectOpenWindow(String),
    RuleNameChanged(String),
    RuleExecChanged(String),
    RuleActionChanged(ActionChoice),
    RuleCloseCountChanged(String),
    RuleWorkspaceChanged(String),
    RuleSpecialWorkspaceChanged(String),
    RuleRepeatCountChanged(String),
    AddRule,
    SetFlag(Flag, bool),
    LaunchDelayChanged(String),
    ApplyLaunchDelay,
    RefreshDiagnostics,
    CheckVersion,
    VersionChecked(Result<(bool, String), String>),
    InstallDesktopEntry,
    DesktopEntryInstalled(Result<(), String>),
    UninstallDesktopEntry,
    DesktopEntryUninstalled(Result<(), String>),
    FixPath,
    DismissInstallBanner,
    DismissAurUpdateBanner,
    DismissUpdateInstalledModal,
    ShowAurUpdateModal,
    DismissAurUpdateModal,
    ShowAurUninstallModal,
    DismissAurUninstallModal,
    ShowPurgeModal,
    DismissPurgeModal,
    ConfirmPurgeAndExit,
    ExportConfig,
    ExportConfigSelected(Option<PathBuf>),
    ImportConfig,
    ImportConfigSelected(Option<PathBuf>),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum DiagStatus {
    Ok,
    Warning,
    Error,
}

#[derive(Clone, Copy, Debug)]
enum DiagAction {
    CheckVersion,
    OpenReleases,
    InstallDesktop,
    UpdateDesktop,
    UninstallDesktop,
    ShowAurUpdate,
    ShowAurUninstall,
    FixPath,
}

struct DiagRow {
    icon: Icon,
    title: String,
    detail: Option<String>,
    status: DiagStatus,
    action: Option<DiagAction>,
    action_disabled: bool,
    label: Option<String>,
}

const INLINE_DETAIL_MAX: usize = 32;

impl DiagRow {
    fn inline_detail(&self) -> Option<&str> {
        self.detail
            .as_deref()
            .filter(|value| value.chars().count() <= INLINE_DETAIL_MAX)
    }

    fn subtitle(&self) -> Option<&str> {
        self.detail
            .as_deref()
            .filter(|value| value.chars().count() > INLINE_DETAIL_MAX)
    }
}

#[derive(Default, Clone)]
struct VersionState {
    checking: bool,
    result: Option<Result<(bool, String), String>>,
}

pub struct App {
    shared: SharedState,
    show_requests: Arc<Mutex<bool>>,
    tray: Option<TrayBridge>,
    page: Page,
    show_welcome: bool,
    dark: bool,
    system_mode: ThemeMode,
    accent: Option<Color>,
    system_colors: Option<SystemColors>,
    window: Option<window::Id>,
    window_open: bool,
    snapshot: AppState,
    switches: [f32; Flag::COUNT],
    add_expanded: bool,
    editing_index: Option<usize>,
    open_windows: Vec<String>,
    new_name: String,
    new_exec: String,
    new_action: ActionChoice,
    new_close_count: String,
    new_workspace: String,
    new_special_ws: String,
    new_repeat_count: String,
    launch_delay_input: String,
    version: VersionState,
    diagnostics: Vec<DiagRow>,
    desktop_status: DesktopIntegrationStatus,
    path_ok: bool,
    show_update_installed_modal: bool,
    show_aur_update_modal: bool,
    show_aur_uninstall_modal: bool,
    show_purge_modal: bool,
    dismissed_install_banner: bool,
    dismissed_aur_update_banner: bool,
    status_error: Option<String>,
}

impl App {
    fn new(
        shared: SharedState,
        show_requests: Arc<Mutex<bool>>,
        start_hidden: bool,
    ) -> (Self, Task<Message>) {
        let mut snapshot = { shared.lock().unwrap().clone() };
        if snapshot.last_run_version.as_deref() != Some(CURRENT_VERSION) {
            let mut s = shared.lock().unwrap();
            s.last_run_version = Some(CURRENT_VERSION.to_string());
            s.save();
            snapshot = s.clone();
        }

        let switches = std::array::from_fn(|index| {
            if flag_value(&snapshot, FLAGS[index]) {
                1.0
            } else {
                0.0
            }
        });

        let system_dark = system_dark_preference();
        let system_mode = if system_dark.unwrap_or(false) {
            ThemeMode::Dark
        } else {
            ThemeMode::Light
        };
        let dark = if snapshot.theme_sync {
            system_dark.unwrap_or(false)
        } else {
            snapshot.theme_dark
        };
        let accent = snapshot.theme_sync.then(|| system_accent(dark)).flatten();
        let system_colors = snapshot
            .system_colors
            .then(|| system_palette(dark))
            .flatten();

        let desktop_status = crate::backend::check_desktop_integration();
        let show_update_installed_modal = matches!(
            desktop_status,
            DesktopIntegrationStatus::Installed { up_to_date: false }
        );
        let add_expanded = snapshot.apps.is_empty();

        let mut app = Self {
            shared,
            show_requests,
            tray: None,
            page: Page::Rules,
            show_welcome: !snapshot.shown_warning,
            dark,
            system_mode,
            accent,
            system_colors,
            window: None,
            window_open: false,
            launch_delay_input: snapshot.launch_delay.to_string(),
            add_expanded,
            editing_index: None,
            open_windows: hypr::list_open_classes(),
            new_name: String::new(),
            new_exec: String::new(),
            new_action: ActionChoice::HideToTray,
            new_close_count: "2".to_string(),
            new_workspace: "1".to_string(),
            new_special_ws: "shelf".to_string(),
            new_repeat_count: "1".to_string(),
            version: VersionState {
                checking: true,
                result: None,
            },
            diagnostics: Vec::new(),
            desktop_status,
            path_ok: crate::backend::is_in_path(),
            show_update_installed_modal,
            show_aur_update_modal: false,
            show_aur_uninstall_modal: false,
            show_purge_modal: false,
            dismissed_install_banner: false,
            dismissed_aur_update_banner: false,
            status_error: None,
            snapshot,
            switches,
        };

        app.refresh_diagnostics();
        app.tray = TrayBridge::spawn(app.shared.clone());

        let mut tasks = vec![
            iced::system::theme().map(Message::ThemeModeChanged),
            Task::perform(check_latest_version(), Message::VersionChecked),
        ];
        if !start_hidden {
            tasks.push(app.show_window());
        }
        (app, Task::batch(tasks))
    }

    fn show_window(&mut self) -> Task<Message> {
        if self.window_open
            && let Some(id) = self.window
        {
            return window::gain_focus(id);
        }
        let (id, open) = window::open(window_settings());
        self.window = Some(id);
        self.window_open = true;
        open.map(Message::WindowOpened)
    }

    fn theme(&self) -> Theme {
        custom_theme(self.dark, self.accent, self.system_colors)
    }

    fn refresh_theme(&mut self) {
        self.dark = if self.snapshot.theme_sync {
            system_dark_preference().unwrap_or(self.system_mode == ThemeMode::Dark)
        } else {
            self.snapshot.theme_dark
        };
        let dark = self.dark;
        self.accent = self
            .snapshot
            .theme_sync
            .then(|| system_accent(dark))
            .flatten();
        self.system_colors = self
            .snapshot
            .system_colors
            .then(|| system_palette(dark))
            .flatten();
    }

    fn sync(&mut self) {
        let state = self.shared.lock().unwrap();
        self.snapshot = state.clone();
    }

    fn mutate(&mut self, f: impl FnOnce(&mut AppState)) {
        {
            let mut state = self.shared.lock().unwrap();
            f(&mut state);
            state.save();
        }
        self.sync();
    }

    fn refresh_diagnostics(&mut self) {
        self.desktop_status = crate::backend::check_desktop_integration();
        self.path_ok = crate::backend::is_in_path();
        self.diagnostics = build_diagnostics(&self.version, &self.desktop_status, self.path_ok);
    }

    fn on_tick(&mut self) -> Task<Message> {
        let mut tasks = self.pump_tray();

        let requested = {
            let mut req = self.show_requests.lock().unwrap();
            let was = *req;
            *req = false;
            was
        };
        if requested {
            tasks.push(self.show_window());
        }

        self.sync();
        if let Some(tray) = &mut self.tray {
            tray.refresh(self.snapshot.apps.len(), &self.snapshot.hidden_apps);
        }

        if tasks.is_empty() {
            Task::none()
        } else {
            Task::batch(tasks)
        }
    }

    fn on_frame(&mut self) {
        for (index, flag) in FLAGS.iter().enumerate() {
            let target = if flag_value(&self.snapshot, *flag) {
                1.0
            } else {
                0.0
            };
            let current = self.switches[index];
            let next = current + (target - current) * 0.3;
            self.switches[index] = if (target - next).abs() < 0.002 {
                target
            } else {
                next
            };
        }
    }

    fn animations_active(&self) -> bool {
        FLAGS.iter().enumerate().any(|(index, flag)| {
            let target = if flag_value(&self.snapshot, *flag) {
                1.0
            } else {
                0.0
            };
            self.switches[index] != target
        })
    }

    fn switch_el<'a>(&self, flag: Flag, enabled: bool, palette: Palette) -> Element<'a, Message> {
        switch(
            flag_value(&self.snapshot, flag),
            enabled,
            self.switches[flag.index()],
            palette,
            move |value| Message::SetFlag(flag, value),
        )
    }

    fn pump_tray(&mut self) -> Vec<Task<Message>> {
        let commands = match &self.tray {
            Some(tray) => tray.drain(),
            None => return Vec::new(),
        };

        let mut tasks = Vec::new();
        for command in commands {
            match command {
                TrayCommand::Show => {
                    tasks.push(self.show_window());
                }
                TrayCommand::ShowApp(app_name) => {
                    hypr::show_from_special(&app_name);
                    let mut s = self.shared.lock().unwrap();
                    s.hidden_apps.retain(|a| a != &app_name);
                }
                TrayCommand::Quit => {
                    tasks.push(iced::exit());
                }
            }
        }
        tasks
    }
}

fn window_settings() -> window::Settings {
    window::Settings {
        size: Size::new(452.0, 604.0),
        min_size: Some(Size::new(380.0, 460.0)),
        resizable: true,
        exit_on_close_request: false,
        ..window::Settings::default()
    }
}

pub fn run(
    shared: SharedState,
    show_requests: Arc<Mutex<bool>>,
    start_hidden: bool,
) -> iced::Result {
    let boot = move || App::new(shared.clone(), show_requests.clone(), start_hidden);

    iced::daemon::daemon(boot, update, view)
        .title(APP_TITLE)
        .theme(|app: &App, _window: window::Id| app.theme())
        .style(|_app: &App, theme: &Theme| iced::theme::Style {
            background_color: theme.palette().background,
            text_color: theme.palette().text,
        })
        .subscription(subscription)
        .run()
}

fn subscription(app: &App) -> Subscription<Message> {
    let mut subscriptions = vec![
        time::every(TICK).map(|_| Message::Tick),
        iced::system::theme_changes().map(Message::ThemeModeChanged),
        window::close_requests().map(Message::CloseRequested),
    ];
    if app.animations_active() {
        subscriptions.push(time::every(FRAME).map(|_| Message::Frame));
    }
    Subscription::batch(subscriptions)
}

fn update(app: &mut App, message: Message) -> Task<Message> {
    match message {
        Message::Tick => app.on_tick(),
        Message::Frame => {
            app.on_frame();
            Task::none()
        }
        Message::ThemeModeChanged(mode) => {
            app.system_mode = mode;
            app.refresh_theme();
            Task::none()
        }
        Message::WindowOpened(id) => {
            app.window = Some(id);
            app.window_open = true;
            Task::none()
        }
        Message::CloseRequested(id) => {
            app.window_open = false;
            window::close(id)
        }
        Message::DismissWelcome => {
            app.show_welcome = false;
            app.mutate(|state| state.shown_warning = true);
            Task::none()
        }
        Message::ToggleSettings => {
            app.status_error = None;
            app.page = match app.page {
                Page::Settings => Page::Rules,
                _ => {
                    app.refresh_diagnostics();
                    Page::Settings
                }
            };
            Task::none()
        }
        Message::BackToRules => {
            app.status_error = None;
            app.page = Page::Rules;
            Task::none()
        }
        Message::OpenUrl(url) => {
            let _ = Command::new("xdg-open").arg(url).spawn();
            Task::none()
        }
        Message::RunAllRules => {
            app.status_error = None;
            if !app.snapshot.apps.is_empty() {
                crate::backend::start_backend(app.shared.clone());
            }
            Task::none()
        }
        Message::RunRule(index) => {
            app.status_error = None;
            if let Some(rule) = app.snapshot.apps.get(index).cloned() {
                let delay = app.snapshot.launch_delay;
                let notify = app.snapshot.notifications;
                let shared = app.shared.clone();
                std::thread::spawn(move || {
                    crate::backend::run_rule(&rule, delay, notify, &shared);
                });
            }
            Task::none()
        }
        Message::DeleteRule(index) => {
            if app.editing_index == Some(index) {
                app.editing_index = None;
                app.new_name.clear();
                app.new_exec.clear();
                app.new_close_count = "2".to_string();
                app.new_workspace = "1".to_string();
                app.new_special_ws = "shelf".to_string();
                app.new_repeat_count = "1".to_string();
            } else if let Some(editing) = app.editing_index
                && index < editing
            {
                app.editing_index = Some(editing - 1);
            }
            app.mutate(|state| {
                if index < state.apps.len() {
                    state.apps.remove(index);
                }
            });
            Task::none()
        }
        Message::StartEditRule(index) => {
            if let Some(rule) = app.snapshot.apps.get(index).cloned() {
                app.editing_index = Some(index);
                app.new_name = rule.name;
                app.new_exec = rule.exec;
                match rule.action {
                    Action::HideToTray => {
                        app.new_action = ActionChoice::HideToTray;
                        app.new_repeat_count = "1".to_string();
                    }
                    Action::HideToTrayTimes(n) => {
                        app.new_action = ActionChoice::HideToTray;
                        app.new_repeat_count = n.to_string();
                    }
                    Action::Close => app.new_action = ActionChoice::Close,
                    Action::Close2 => {
                        app.new_action = ActionChoice::CloseTimes;
                        app.new_close_count = "2".to_string();
                    }
                    Action::CloseTimes(n) => {
                        app.new_action = ActionChoice::CloseTimes;
                        app.new_close_count = n.to_string();
                    }
                    Action::Workspace(ws) => {
                        app.new_action = ActionChoice::Workspace;
                        app.new_workspace = ws.to_string();
                        app.new_repeat_count = "1".to_string();
                    }
                    Action::WorkspaceTimes(ws, n) => {
                        app.new_action = ActionChoice::Workspace;
                        app.new_workspace = ws.to_string();
                        app.new_repeat_count = n.to_string();
                    }
                    Action::SpecialWorkspace(name) => {
                        app.new_action = ActionChoice::SpecialWorkspace;
                        app.new_special_ws = name;
                        app.new_repeat_count = "1".to_string();
                    }
                    Action::SpecialWorkspaceTimes(name, n) => {
                        app.new_action = ActionChoice::SpecialWorkspace;
                        app.new_special_ws = name;
                        app.new_repeat_count = n.to_string();
                    }
                    Action::Float => app.new_action = ActionChoice::Float,
                    Action::Fullscreen => app.new_action = ActionChoice::Fullscreen,
                    Action::LaunchOnly => app.new_action = ActionChoice::LaunchOnly,
                }
                app.open_windows = hypr::list_open_classes();
                app.add_expanded = true;
                app.status_error = None;
            }
            Task::none()
        }
        Message::CancelEditRule => {
            app.editing_index = None;
            app.new_name.clear();
            app.new_exec.clear();
            app.new_action = ActionChoice::HideToTray;
            app.new_close_count = "2".to_string();
            app.new_workspace = "1".to_string();
            app.new_special_ws = "shelf".to_string();
            app.new_repeat_count = "1".to_string();
            app.add_expanded = false;
            app.status_error = None;
            Task::none()
        }
        Message::OpenLogs(name) => {
            app.page = Page::Logs(name);
            Task::none()
        }
        Message::ClearLogs(name) => {
            {
                let mut s = app.shared.lock().unwrap();
                s.logs.remove(&name);
            }
            app.sync();
            Task::none()
        }
        Message::ToggleAddRule => {
            app.add_expanded = !app.add_expanded;
            if app.add_expanded {
                app.open_windows = hypr::list_open_classes();
            } else {
                app.editing_index = None;
            }
            Task::none()
        }
        Message::SelectOpenWindow(class) => {
            if app.new_exec.trim().is_empty() {
                app.new_exec = class.to_lowercase();
            }
            app.new_name = class;
            Task::none()
        }
        Message::RuleNameChanged(value) => {
            app.new_name = value;
            Task::none()
        }
        Message::RuleExecChanged(value) => {
            app.new_exec = value;
            Task::none()
        }
        Message::RuleActionChanged(action) => {
            app.new_action = action;
            Task::none()
        }
        Message::RuleCloseCountChanged(value) => {
            app.new_close_count = value;
            Task::none()
        }
        Message::RuleWorkspaceChanged(value) => {
            app.new_workspace = value;
            Task::none()
        }
        Message::RuleSpecialWorkspaceChanged(value) => {
            app.new_special_ws = value;
            Task::none()
        }
        Message::RuleRepeatCountChanged(value) => {
            app.new_repeat_count = value;
            Task::none()
        }
        Message::AddRule => {
            let name = app.new_name.trim().to_string();
            let exec = app.new_exec.trim().to_string();
            if name.is_empty() || exec.is_empty() {
                app.status_error = Some("Window Class and Exec Command are required".to_string());
                return Task::none();
            }
            app.status_error = None;
            let repeat = app
                .new_repeat_count
                .trim()
                .parse::<u32>()
                .unwrap_or(1)
                .clamp(1, 20);
            let action = match app.new_action {
                ActionChoice::HideToTray => {
                    if repeat > 1 {
                        Action::HideToTrayTimes(repeat)
                    } else {
                        Action::HideToTray
                    }
                }
                ActionChoice::Close => Action::Close,
                ActionChoice::CloseTimes => {
                    let count = app
                        .new_close_count
                        .trim()
                        .parse::<u32>()
                        .unwrap_or(2)
                        .clamp(1, 20);
                    Action::CloseTimes(count)
                }
                ActionChoice::Workspace => {
                    let ws = app.new_workspace.trim().parse::<u32>().unwrap_or(1).max(1);
                    if repeat > 1 {
                        Action::WorkspaceTimes(ws, repeat)
                    } else {
                        Action::Workspace(ws)
                    }
                }
                ActionChoice::SpecialWorkspace => {
                    let trimmed = app.new_special_ws.trim();
                    let ws = trimmed.strip_prefix("special:").unwrap_or(trimmed);
                    let ws = if ws.is_empty() { "shelf" } else { ws };
                    if repeat > 1 {
                        Action::SpecialWorkspaceTimes(ws.to_string(), repeat)
                    } else {
                        Action::SpecialWorkspace(ws.to_string())
                    }
                }
                ActionChoice::Float => Action::Float,
                ActionChoice::Fullscreen => Action::Fullscreen,
                ActionChoice::LaunchOnly => Action::LaunchOnly,
            };
            let rule = AppRule { name, exec, action };
            let edit_idx = app.editing_index.take();
            app.mutate(|state| {
                if let Some(idx) = edit_idx
                    && idx < state.apps.len()
                {
                    state.apps[idx] = rule;
                } else {
                    state.apps.push(rule);
                }
            });
            app.new_name.clear();
            app.new_exec.clear();
            app.new_action = ActionChoice::HideToTray;
            app.new_close_count = "2".to_string();
            app.new_workspace = "1".to_string();
            app.new_special_ws = "shelf".to_string();
            app.new_repeat_count = "1".to_string();
            app.add_expanded = false;
            Task::none()
        }
        Message::SetFlag(flag, value) => {
            if flag == Flag::AutoStart && value && !crate::backend::desktop_file_exists() {
                app.status_error = Some("Install Desktop Integration first".to_string());
                return Task::none();
            }
            app.status_error = None;
            app.mutate(|state| flag.apply(state, value));
            if flag == Flag::AutoStart {
                crate::backend::autostart(value);
            }
            if matches!(flag, Flag::ThemeSync | Flag::ThemeDark | Flag::SystemColors) {
                app.refresh_theme();
            }
            Task::none()
        }
        Message::LaunchDelayChanged(value) => {
            app.launch_delay_input = value.clone();
            if let Ok(ms) = value.trim().parse::<u64>() {
                let clamped = ms.min(30_000);
                app.mutate(|state| state.launch_delay = clamped);
            }
            Task::none()
        }
        Message::ApplyLaunchDelay => {
            let ms = app
                .launch_delay_input
                .trim()
                .parse::<u64>()
                .unwrap_or(app.snapshot.launch_delay)
                .min(30_000);
            app.launch_delay_input = ms.to_string();
            app.mutate(|state| state.launch_delay = ms);
            Task::none()
        }
        Message::RefreshDiagnostics => {
            app.refresh_diagnostics();
            Task::none()
        }
        Message::CheckVersion => {
            if app.version.checking {
                return Task::none();
            }
            app.version.checking = true;
            app.version.result = None;
            app.refresh_diagnostics();
            Task::perform(check_latest_version(), Message::VersionChecked)
        }
        Message::VersionChecked(result) => {
            app.version.checking = false;
            app.version.result = Some(result);
            app.refresh_diagnostics();
            Task::none()
        }
        Message::InstallDesktopEntry => Task::perform(
            async { crate::backend::install_desktop_entry() },
            Message::DesktopEntryInstalled,
        ),
        Message::DesktopEntryInstalled(result) => {
            app.show_update_installed_modal = false;
            match result {
                Ok(()) => {
                    app.status_error = None;
                    app.mutate(|state| state.desktop_installed = true);
                }
                Err(err) => {
                    app.status_error = Some(err);
                }
            }
            app.refresh_diagnostics();
            Task::none()
        }
        Message::UninstallDesktopEntry => Task::perform(
            async { crate::backend::uninstall_desktop_entry() },
            Message::DesktopEntryUninstalled,
        ),
        Message::DesktopEntryUninstalled(result) => {
            match result {
                Ok(()) => {
                    app.status_error = None;
                    app.mutate(|state| state.desktop_installed = false);
                    app.dismissed_install_banner = false;
                }
                Err(err) => {
                    app.status_error = Some(err);
                }
            }
            app.refresh_diagnostics();
            Task::none()
        }
        Message::FixPath => {
            if !crate::backend::add_to_path_config() {
                app.status_error = Some("Failed to update shell config".to_string());
            }
            app.refresh_diagnostics();
            Task::none()
        }
        Message::DismissInstallBanner => {
            app.dismissed_install_banner = true;
            Task::none()
        }
        Message::DismissAurUpdateBanner => {
            app.dismissed_aur_update_banner = true;
            Task::none()
        }
        Message::DismissUpdateInstalledModal => {
            app.show_update_installed_modal = false;
            Task::none()
        }
        Message::ShowAurUpdateModal => {
            app.show_aur_update_modal = true;
            Task::none()
        }
        Message::DismissAurUpdateModal => {
            app.show_aur_update_modal = false;
            Task::none()
        }
        Message::ShowAurUninstallModal => {
            app.show_aur_uninstall_modal = true;
            Task::none()
        }
        Message::DismissAurUninstallModal => {
            app.show_aur_uninstall_modal = false;
            Task::none()
        }
        Message::ShowPurgeModal => {
            app.show_purge_modal = true;
            Task::none()
        }
        Message::DismissPurgeModal => {
            app.show_purge_modal = false;
            Task::none()
        }
        Message::ConfirmPurgeAndExit => {
            let _ = crate::backend::uninstall();
            std::process::exit(0);
        }
        Message::ExportConfig => Task::perform(pick_save_file(), Message::ExportConfigSelected),
        Message::ExportConfigSelected(path) => {
            if let Some(path) = path {
                let res = {
                    let s = app.shared.lock().unwrap();
                    s.export_config(path)
                };
                if let Err(e) = res {
                    app.status_error = Some(e);
                }
            }
            Task::none()
        }
        Message::ImportConfig => Task::perform(pick_open_file(), Message::ImportConfigSelected),
        Message::ImportConfigSelected(path) => {
            if let Some(path) = path {
                let res = {
                    let mut s = app.shared.lock().unwrap();
                    s.import_config(path)
                };
                app.sync();
                app.launch_delay_input = app.snapshot.launch_delay.to_string();
                app.refresh_theme();
                if let Err(e) = res {
                    app.status_error = Some(e);
                }
            }
            Task::none()
        }
    }
}

fn view(app: &App, _window: window::Id) -> Element<'_, Message> {
    let theme = app.theme();
    let mut palette = Palette::of(&theme);
    if let Some(system) = app.system_colors {
        palette.card = system.card;
        palette.strip = mix(theme.palette().background, system.card, 0.5);
    }

    if app.show_welcome {
        return container(welcome_page(palette))
            .width(Fill)
            .height(Fill)
            .padding(24.0)
            .into();
    }

    let page = match &app.page {
        Page::Rules => rules_page(app, palette),
        Page::Settings => settings_page(app, palette),
        Page::Logs(name) => logs_page(app, name, palette),
    };

    let mut content = column![
        header(palette),
        divider(palette),
        scrollable(container(page).width(Fill).padding(Padding {
            top: 16.0,
            bottom: 16.0,
            left: 16.0,
            right: 16.0,
        }))
        .width(Fill)
        .height(Fill),
    ];

    if let Some(strip) = status_strip(app, palette) {
        content = content.push(strip);
    }

    content = content.push(bottom_bar(app, palette));

    let main_view = container(content.width(Fill).height(Fill))
        .width(Fill)
        .height(Fill);

    let mut stacked = stack![main_view];
    if app.show_update_installed_modal {
        stacked = stacked.push(update_installed_modal(palette));
    }
    if app.show_aur_update_modal
        && let DesktopIntegrationStatus::Aur { package, helper } = &app.desktop_status
    {
        let latest_str = match &app.version.result {
            Some(Ok((false, latest))) => latest.as_str(),
            _ => "",
        };
        stacked = stacked.push(aur_update_modal(package, helper, latest_str, palette));
    }
    if app.show_aur_uninstall_modal
        && let DesktopIntegrationStatus::Aur { package, helper } = &app.desktop_status
    {
        stacked = stacked.push(aur_uninstall_modal(package, helper, palette));
    }
    if app.show_purge_modal {
        let (is_aur, aur_pkg, aur_helper) = match &app.desktop_status {
            DesktopIntegrationStatus::Aur { package, helper } => {
                (true, Some(package.as_str()), Some(helper.as_str()))
            }
            _ => (false, None, None),
        };
        stacked = stacked.push(purge_modal(is_aur, aur_pkg, aur_helper, palette));
    }

    stacked.width(Fill).height(Fill).into()
}

fn status_strip(app: &App, palette: Palette) -> Option<Element<'_, Message>> {
    if matches!(app.page, Page::Settings) && app.status_error.is_none() {
        return None;
    }

    let (status, color) = if let Some(err) = &app.status_error {
        (err.clone(), palette.warning)
    } else if app.snapshot.running_rules > 0 {
        (
            format!("Launching {} rule(s)", app.snapshot.running_rules),
            palette.success,
        )
    } else {
        (
            format!(
                "{} rules • {} hidden",
                app.snapshot.apps.len(),
                app.snapshot.hidden_apps.len()
            ),
            palette.muted,
        )
    };

    Some(
        container(text(status).size(11.5).color(color))
            .width(Fill)
            .height(28.0)
            .align_x(Horizontal::Center)
            .align_y(Vertical::Center)
            .style(move |_theme| container::Style {
                background: Some(Background::Color(palette.strip)),
                border: Border::default(),
                ..container::Style::default()
            })
            .into(),
    )
}

fn tip<'a>(
    content: impl Into<Element<'a, Message>>,
    label: impl Into<Element<'a, Message>>,
    position: TooltipPosition,
    palette: Palette,
) -> Element<'a, Message> {
    tooltip(content, label, position)
        .gap(6.0)
        .padding(6.0)
        .style(move |_theme| tooltip_surface(palette))
        .into()
}

fn tooltip_surface(palette: Palette) -> container::Style {
    container::Style {
        text_color: Some(palette.fg),
        background: Some(Background::Color(palette.card)),
        border: Border {
            width: 1.0,
            color: palette.border,
            radius: 8.0.into(),
        },
        shadow: Shadow::default(),
        snap: false,
    }
}

fn title_row<'a>(size: f32, palette: Palette) -> iced::widget::Row<'a, Message> {
    row![
        text("To").size(size).font(BOLD),
        text("Tray").size(size).font(BOLD).color(palette.accent),
    ]
    .spacing(0.0)
    .align_y(Vertical::Center)
}

fn header(palette: Palette) -> Element<'static, Message> {
    let title = title_row(14.0, palette);

    let buttons = row![
        tip(
            icon_button(Icon::Repository, palette, Message::OpenUrl(REPOSITORY_URL)),
            text("Open GitHub repository").size(11.0),
            TooltipPosition::Bottom,
            palette,
        ),
        tip(
            icon_button(Icon::Settings, palette, Message::ToggleSettings),
            text("Settings & Diagnostics").size(11.0),
            TooltipPosition::Bottom,
            palette,
        ),
    ]
    .spacing(4.0)
    .align_y(Vertical::Center);

    let content = row![title, Space::new().width(Fill), buttons]
        .align_y(Vertical::Center)
        .width(Fill);

    container(content)
        .width(Fill)
        .padding(Padding {
            top: 7.0,
            bottom: 7.0,
            left: 16.0,
            right: 14.0,
        })
        .into()
}

fn welcome_page(palette: Palette) -> Element<'static, Message> {
    let title = title_row(24.0, palette);

    let note =
        container(
            column![
            text("Hyprland Only")
                .size(13.5)
                .font(SEMIBOLD)
                .align_x(Horizontal::Center)
                .width(Fill),
            text("This tool is designed specifically for Hyprland using hyprctl dispatch commands.")
                .size(12.0)
                .color(palette.muted)
                .align_x(Horizontal::Center)
                .width(Fill),
        ]
            .spacing(8.0)
            .align_x(Horizontal::Center)
            .width(Fill),
        )
        .padding(14.0)
        .width(Fill)
        .style(move |_theme| container::Style {
            background: Some(Background::Color(palette.surface)),
            border: Border {
                color: palette.border,
                width: 1.0,
                radius: 12.0.into(),
            },
            ..container::Style::default()
        });

    let links = row![
        button(
            row![
                icon(Icon::Repository, 14.0, palette.fg),
                text("GitHub").size(12.0).font(SEMIBOLD),
            ]
            .spacing(6.0)
            .align_y(Vertical::Center),
        )
        .on_press(Message::OpenUrl(REPOSITORY_URL))
        .style(styled(ButtonKind::Subtle, palette)),
    ]
    .spacing(10.0)
    .align_y(Vertical::Center);

    column![
        Space::new().height(Fill),
        icon(Icon::Warning, 48.0, palette.warning),
        title,
        text("Auto-start & Tray Manager")
            .size(13.5)
            .font(BOLD)
            .color(palette.accent),
        Space::new().height(14.0),
        note,
        Space::new().height(10.0),
        links,
        Space::new().height(Fill),
        button(
            container(text("Get Started").size(14.0).font(SEMIBOLD))
                .width(Fill)
                .height(Fill)
                .center_x(Fill)
                .center_y(Fill),
        )
        .width(Fill)
        .height(42.0)
        .padding(0.0)
        .on_press(Message::DismissWelcome)
        .style(styled(ButtonKind::Primary, palette)),
    ]
    .spacing(6.0)
    .align_x(Horizontal::Center)
    .width(Fill)
    .height(Fill)
    .into()
}

fn rules_page(app: &App, palette: Palette) -> Element<'_, Message> {
    let mut page_col = column![].width(Fill).spacing(10.0);

    if !crate::backend::is_hyprland() {
        page_col = page_col.push(
            container(
                row![
                    icon(Icon::Warning, 14.0, palette.warning),
                    text("Hyprland session not detected.")
                        .size(11.5)
                        .color(palette.warning),
                ]
                .spacing(8.0)
                .align_y(Vertical::Center),
            )
            .width(Fill)
            .padding(10.0)
            .style(move |_theme| container::Style {
                background: Some(Background::Color(palette.card)),
                border: Border {
                    color: palette.hairline,
                    width: 1.0,
                    radius: 10.0.into(),
                },
                ..container::Style::default()
            }),
        );
    }

    if !app.dismissed_aur_update_banner
        && let (DesktopIntegrationStatus::Aur { package, helper }, Some(Ok((false, latest)))) =
            (&app.desktop_status, &app.version.result)
    {
        page_col = page_col.push(
            container(
                row![
                    icon(Icon::Info, 16.0, palette.accent),
                    column![
                        text(format!("Update v{latest} available (AUR)"))
                            .size(12.5)
                            .font(SEMIBOLD),
                        text(format!("Run '{helper} -S {package}' in terminal"))
                            .size(11.0)
                            .color(palette.muted),
                    ]
                    .spacing(2.0)
                    .width(Fill),
                    button(text("How to Update").size(11.0))
                        .padding(Padding {
                            top: 5.0,
                            bottom: 5.0,
                            left: 10.0,
                            right: 10.0,
                        })
                        .style(styled(ButtonKind::Subtle, palette))
                        .on_press(Message::ShowAurUpdateModal),
                    tip(
                        icon_button(Icon::Close, palette, Message::DismissAurUpdateBanner),
                        text("Dismiss").size(11.0),
                        TooltipPosition::Bottom,
                        palette,
                    ),
                ]
                .spacing(8.0)
                .align_y(Vertical::Center)
                .width(Fill),
            )
            .width(Fill)
            .padding(10.0)
            .style(move |_theme| container::Style {
                background: Some(Background::Color(palette.card)),
                border: Border {
                    color: palette.hairline,
                    width: 1.0,
                    radius: 10.0.into(),
                },
                ..container::Style::default()
            }),
        );
    }

    if !app.dismissed_install_banner
        && matches!(app.desktop_status, DesktopIntegrationStatus::NotInstalled)
    {
        page_col = page_col.push(
            container(
                row![
                    icon(Icon::Layers, 16.0, palette.accent),
                    column![
                        text("Desktop Integration").size(12.5).font(SEMIBOLD),
                        text("Add to applications menu and desktop launcher")
                            .size(11.0)
                            .color(palette.muted),
                    ]
                    .spacing(2.0)
                    .width(Fill),
                    button(text("Add to System").size(11.0).font(SEMIBOLD))
                        .padding(Padding {
                            top: 5.0,
                            bottom: 5.0,
                            left: 12.0,
                            right: 12.0,
                        })
                        .style(styled(ButtonKind::Primary, palette))
                        .on_press(Message::InstallDesktopEntry),
                    tip(
                        icon_button(Icon::Close, palette, Message::DismissInstallBanner),
                        text("Dismiss").size(11.0),
                        TooltipPosition::Bottom,
                        palette,
                    ),
                ]
                .spacing(8.0)
                .align_y(Vertical::Center)
                .width(Fill),
            )
            .width(Fill)
            .padding(10.0)
            .style(move |_theme| container::Style {
                background: Some(Background::Color(palette.card)),
                border: Border {
                    color: palette.hairline,
                    width: 1.0,
                    radius: 10.0.into(),
                },
                ..container::Style::default()
            }),
        );
    }

    let mut rules_list = column![].width(Fill).spacing(0.0);
    if app.snapshot.apps.is_empty() {
        rules_list = rules_list.push(
            container(
                text("No rules defined yet.")
                    .size(12.0)
                    .color(palette.muted),
            )
            .width(Fill)
            .padding(18.0)
            .align_x(Horizontal::Center),
        );
    } else {
        for (idx, rule) in app.snapshot.apps.iter().enumerate() {
            if idx > 0 {
                rules_list = rules_list.push(divider(palette));
            }
            rules_list = rules_list.push(rule_row(idx, rule, palette));
        }
    }

    page_col = page_col.push(card(rules_list, palette));

    let is_editing = app.editing_index.is_some();
    let header_title = if is_editing {
        "Edit Rule"
    } else {
        "Add New Rule"
    };

    let mut header_row = row![
        icon_tile(if is_editing { Icon::Edit } else { Icon::Plus }, palette),
        text(header_title).size(13.0).font(SEMIBOLD).width(Fill),
    ]
    .spacing(10.0)
    .align_y(Vertical::Center)
    .width(Fill);

    if is_editing {
        header_row = header_row.push(
            button(text("Cancel").size(11.0))
                .padding(Padding {
                    top: 4.0,
                    bottom: 4.0,
                    left: 10.0,
                    right: 10.0,
                })
                .style(styled(ButtonKind::Subtle, palette))
                .on_press(Message::CancelEditRule),
        );
    }

    let add_header = button(header_row)
        .width(Fill)
        .padding(Padding {
            top: 5.0,
            bottom: 5.0,
            left: 0.0,
            right: 0.0,
        })
        .style(transparent_button_style(palette))
        .on_press(Message::ToggleAddRule);

    let mut add_col = column![add_header].width(Fill).spacing(0.0);

    if app.add_expanded {
        let action_pick = pick_list(
            &ACTION_CHOICES[..],
            Some(app.new_action),
            Message::RuleActionChanged,
        )
        .text_size(13.0)
        .padding(Padding {
            top: 6.0,
            bottom: 6.0,
            left: 10.0,
            right: 8.0,
        })
        .width(Length::Fixed(132.0))
        .style(select_style(palette));

        let class_control: Element<'_, Message> = if app.open_windows.is_empty() {
            text_input("firefox", app.new_name.as_str())
                .on_input(Message::RuleNameChanged)
                .size(13.0)
                .padding(Padding {
                    top: 6.0,
                    bottom: 6.0,
                    left: 8.0,
                    right: 8.0,
                })
                .width(Length::Fixed(148.0))
                .style(input_style(palette))
                .into()
        } else {
            row![
                pick_list(
                    app.open_windows.as_slice(),
                    None::<String>,
                    Message::SelectOpenWindow,
                )
                .placeholder("Open...")
                .text_size(12.0)
                .padding(Padding {
                    top: 6.0,
                    bottom: 6.0,
                    left: 8.0,
                    right: 6.0,
                })
                .width(Length::Fixed(86.0))
                .style(select_style(palette)),
                text_input("firefox", app.new_name.as_str())
                    .on_input(Message::RuleNameChanged)
                    .size(13.0)
                    .padding(Padding {
                        top: 6.0,
                        bottom: 6.0,
                        left: 8.0,
                        right: 8.0,
                    })
                    .width(Length::Fixed(114.0))
                    .style(input_style(palette)),
            ]
            .spacing(6.0)
            .align_y(Vertical::Center)
            .into()
        };

        add_col = add_col
            .push(divider(palette))
            .push(settings_row(
                palette,
                Icon::Layers,
                "Window Class",
                None,
                class_control,
                Some(InfoTip::plain(
                    "Pick an open window or enter its class ('hyprctl clients').",
                )),
            ))
            .push(divider(palette))
            .push(settings_row(
                palette,
                Icon::Terminal,
                "Exec Command",
                None,
                text_input("firefox", app.new_exec.as_str())
                    .on_input(Message::RuleExecChanged)
                    .size(13.0)
                    .padding(Padding {
                        top: 6.0,
                        bottom: 6.0,
                        left: 8.0,
                        right: 8.0,
                    })
                    .width(Length::Fixed(148.0))
                    .style(input_style(palette))
                    .into(),
                None,
            ))
            .push(divider(palette))
            .push(settings_row(
                palette,
                Icon::Action,
                "Action",
                None,
                action_pick.into(),
                None,
            ));

        if app.new_action == ActionChoice::CloseTimes {
            add_col = add_col.push(divider(palette)).push(settings_sub_row(
                palette,
                Icon::Close,
                "Close count",
                None,
                text_input("2", app.new_close_count.as_str())
                    .on_input(Message::RuleCloseCountChanged)
                    .size(13.0)
                    .padding(Padding {
                        top: 6.0,
                        bottom: 6.0,
                        left: 8.0,
                        right: 8.0,
                    })
                    .width(Length::Fixed(64.0))
                    .style(input_style(palette))
                    .into(),
                Some(InfoTip::plain(
                    "Number of windows to close sequentially (1-20).",
                )),
            ));
        }

        if app.new_action == ActionChoice::Workspace {
            add_col = add_col.push(divider(palette)).push(settings_sub_row(
                palette,
                Icon::Layers,
                "Workspace #",
                None,
                text_input("1", app.new_workspace.as_str())
                    .on_input(Message::RuleWorkspaceChanged)
                    .size(13.0)
                    .padding(Padding {
                        top: 6.0,
                        bottom: 6.0,
                        left: 8.0,
                        right: 8.0,
                    })
                    .width(Length::Fixed(64.0))
                    .style(input_style(palette))
                    .into(),
                None,
            ));
        }

        if app.new_action == ActionChoice::SpecialWorkspace {
            add_col = add_col.push(divider(palette)).push(settings_sub_row(
                palette,
                Icon::Hide,
                "Special name",
                None,
                text_input("shelf", app.new_special_ws.as_str())
                    .on_input(Message::RuleSpecialWorkspaceChanged)
                    .size(13.0)
                    .padding(Padding {
                        top: 6.0,
                        bottom: 6.0,
                        left: 8.0,
                        right: 8.0,
                    })
                    .width(Length::Fixed(110.0))
                    .style(input_style(palette))
                    .into(),
                Some(InfoTip::plain(
                    "Hyprland special workspace name (e.g. 'shelf' for special:shelf).",
                )),
            ));
        }

        if matches!(
            app.new_action,
            ActionChoice::HideToTray | ActionChoice::Workspace | ActionChoice::SpecialWorkspace
        ) {
            add_col = add_col.push(divider(palette)).push(settings_sub_row(
                palette,
                Icon::Interval,
                "Repeat",
                None,
                text_input("1", app.new_repeat_count.as_str())
                    .on_input(Message::RuleRepeatCountChanged)
                    .size(13.0)
                    .padding(Padding {
                        top: 6.0,
                        bottom: 6.0,
                        left: 8.0,
                        right: 8.0,
                    })
                    .width(Length::Fixed(64.0))
                    .style(input_style(palette))
                    .into(),
                Some(InfoTip::plain(
                    "Number of windows to move sequentially (1-20).",
                )),
            ));
        }

        add_col = add_col.push(
            container(
                button(
                    container(
                        text(if is_editing { "Save Rule" } else { "Add Rule" })
                            .size(13.0)
                            .font(SEMIBOLD),
                    )
                    .width(Fill)
                    .align_x(Horizontal::Center),
                )
                .width(Fill)
                .padding(Padding {
                    top: 8.0,
                    bottom: 8.0,
                    left: 12.0,
                    right: 12.0,
                })
                .style(styled(ButtonKind::Primary, palette))
                .on_press(Message::AddRule),
            )
            .width(Fill)
            .padding(Padding {
                top: 8.0,
                bottom: 6.0,
                left: 0.0,
                right: 0.0,
            }),
        );
    }

    page_col = page_col.push(card(add_col, palette));
    page_col.into()
}

fn rule_row<'a>(index: usize, rule: &'a AppRule, palette: Palette) -> Element<'a, Message> {
    let (action_icon, action_label) = match rule.action {
        Action::HideToTray => (Icon::Hide, "To Tray".to_string()),
        Action::HideToTrayTimes(n) => (Icon::Hide, format!("To Tray ({n}x)")),
        Action::Close => (Icon::WindowClose, "Close".to_string()),
        Action::Close2 => (Icon::WindowClose, "Close 2x".to_string()),
        Action::CloseTimes(n) => (Icon::WindowClose, format!("Close {n}x")),
        Action::Workspace(ws) => (Icon::Layers, format!("WS {ws}")),
        Action::WorkspaceTimes(ws, n) => (Icon::Layers, format!("WS {ws} ({n}x)")),
        Action::SpecialWorkspace(ref name) => {
            let ws = name.trim().strip_prefix("special:").unwrap_or(name.trim());
            let label = if ws.is_empty() {
                "special".to_string()
            } else {
                format!("special:{ws}")
            };
            (Icon::Hide, label)
        }
        Action::SpecialWorkspaceTimes(ref name, n) => {
            let ws = name.trim().strip_prefix("special:").unwrap_or(name.trim());
            let label = if ws.is_empty() {
                format!("special ({n}x)")
            } else {
                format!("special:{ws} ({n}x)")
            };
            (Icon::Hide, label)
        }
        Action::Float => (Icon::Float, "Float".to_string()),
        Action::Fullscreen => (Icon::Fullscreen, "Fullscreen".to_string()),
        Action::LaunchOnly => (Icon::Action, "Launch".to_string()),
    };

    let labels = column![
        text(&rule.name).size(13.0),
        text(format!("{} • {}", rule.exec, action_label))
            .size(11.0)
            .color(palette.muted),
    ]
    .width(Fill)
    .spacing(2.0);

    let controls = row![
        tip(
            icon_button(Icon::Action, palette, Message::RunRule(index)),
            text("Run now").size(11.0),
            TooltipPosition::Top,
            palette,
        ),
        tip(
            icon_button(
                Icon::Terminal,
                palette,
                Message::OpenLogs(rule.name.clone())
            ),
            text("Console").size(11.0),
            TooltipPosition::Top,
            palette,
        ),
        tip(
            icon_button(Icon::Edit, palette, Message::StartEditRule(index)),
            text("Edit").size(11.0),
            TooltipPosition::Top,
            palette,
        ),
        tip(
            icon_button(Icon::Trash, palette, Message::DeleteRule(index)),
            text("Delete").size(11.0),
            TooltipPosition::Top,
            palette,
        ),
    ]
    .spacing(4.0)
    .align_y(Vertical::Center);

    container(
        row![icon_tile(action_icon, palette), labels, controls]
            .spacing(10.0)
            .align_y(Vertical::Center)
            .width(Fill),
    )
    .width(Fill)
    .padding(Padding {
        top: 5.0,
        bottom: 5.0,
        left: 0.0,
        right: 0.0,
    })
    .into()
}

fn logs_page<'a>(app: &'a App, name: &'a str, palette: Palette) -> Element<'a, Message> {
    let logs = app.snapshot.logs.get(name);
    let has_logs = logs.is_some_and(|l| !l.is_empty());

    let header_card = container(
        row![
            icon_tile(Icon::Terminal, palette),
            text(format!("Console - {name}"))
                .size(13.5)
                .font(SEMIBOLD)
                .width(Fill),
            button(text("Clear").size(11.0))
                .padding(Padding {
                    top: 4.0,
                    bottom: 4.0,
                    left: 10.0,
                    right: 10.0,
                })
                .style(styled(ButtonKind::Subtle, palette))
                .on_press_maybe(has_logs.then(|| Message::ClearLogs(name.to_string()))),
        ]
        .spacing(10.0)
        .width(Fill)
        .align_y(Vertical::Center),
    )
    .width(Fill)
    .padding(8.0)
    .style(move |_theme| container::Style {
        background: Some(Background::Color(palette.card)),
        border: Border {
            color: palette.hairline,
            width: 1.0,
            radius: 14.0.into(),
        },
        ..container::Style::default()
    });

    let body: Element<'_, Message> = if let Some(lines) = logs
        && !lines.is_empty()
    {
        let mut col = column![].spacing(3.0).width(Fill);
        for line in lines {
            let color = if line.starts_with("[ERR]") {
                palette.danger
            } else if line.starts_with("[ToTray]") {
                palette.accent
            } else {
                palette.fg
            };
            col = col.push(text(line).size(11.5).font(MONOSPACE).color(color));
        }
        col.into()
    } else {
        container(text("No logs yet.").size(12.0).color(palette.muted))
            .width(Fill)
            .padding(20.0)
            .align_x(Horizontal::Center)
            .into()
    };

    column![header_card, card(container(body).padding(8.0), palette)]
        .spacing(10.0)
        .width(Fill)
        .into()
}

fn settings_page(app: &App, palette: Palette) -> Element<'_, Message> {
    let has_errors = app
        .diagnostics
        .iter()
        .any(|row| row.status == DiagStatus::Error);
    let (overall, overall_color) = if has_errors {
        ("Action Required", palette.danger)
    } else {
        ("Ready", palette.success)
    };

    let diag_header = container(
        row![
            icon_tile(
                if has_errors {
                    Icon::Warning
                } else {
                    Icon::Info
                },
                palette
            ),
            text("Diagnostics").size(13.5).font(SEMIBOLD),
            Space::new().width(Fill),
            text(overall).size(11.5).color(overall_color),
            button(text("Refresh").size(11.0))
                .padding(Padding {
                    top: 4.0,
                    bottom: 4.0,
                    left: 10.0,
                    right: 10.0,
                })
                .style(styled(ButtonKind::Subtle, palette))
                .on_press(Message::RefreshDiagnostics),
        ]
        .spacing(10.0)
        .width(Fill)
        .align_y(Vertical::Center),
    )
    .width(Fill)
    .padding(Padding {
        top: 6.0,
        bottom: 6.0,
        left: 8.0,
        right: 8.0,
    })
    .style(move |_theme| container::Style {
        background: Some(Background::Color(palette.card)),
        border: Border {
            color: palette.hairline,
            width: 1.0,
            radius: 14.0.into(),
        },
        shadow: Shadow {
            color: alpha(Color::BLACK, 0.10),
            blur_radius: 12.0,
            ..Shadow::default()
        },
        ..container::Style::default()
    });

    let mut diag_list = column![].width(Fill).spacing(0.0);
    for (index, entry) in app.diagnostics.iter().enumerate() {
        if index > 0 {
            diag_list = diag_list.push(divider(palette));
        }
        diag_list = diag_list.push(diagnostics_row(entry, palette));
    }

    let settings_card = card(
        column![
            settings_row(
                palette,
                Icon::Autostart,
                "Auto-Start",
                None,
                app.switch_el(Flag::AutoStart, true, palette),
                Some(InfoTip::plain("Launch as a systemd user service.")),
            ),
            divider(palette),
            settings_row(
                palette,
                Icon::Notifications,
                "Notifications",
                None,
                app.switch_el(Flag::Notifications, true, palette),
                None,
            ),
            divider(palette),
            settings_row(
                palette,
                Icon::Hide,
                "Silent Mode",
                None,
                app.switch_el(Flag::SilentMode, true, palette),
                Some(InfoTip::plain("Start hidden in the system tray.")),
            ),
            divider(palette),
            settings_row(
                palette,
                Icon::Multi,
                "Multi-Launch",
                None,
                app.switch_el(Flag::MultiLaunch, true, palette),
                Some(InfoTip::plain("Launch all rules in parallel.")),
            ),
            divider(palette),
            settings_row(
                palette,
                Icon::Interval,
                "Launch Delay",
                None,
                row![
                    text_input("0", app.launch_delay_input.as_str())
                        .on_input(Message::LaunchDelayChanged)
                        .on_submit(Message::ApplyLaunchDelay)
                        .size(13.0)
                        .padding(Padding {
                            top: 6.0,
                            bottom: 6.0,
                            left: 8.0,
                            right: 8.0,
                        })
                        .width(Length::Fixed(64.0))
                        .style(input_style(palette)),
                    text("ms").size(12.0).color(palette.muted),
                ]
                .spacing(6.0)
                .align_y(Vertical::Center)
                .into(),
                None,
            ),
        ]
        .width(Fill),
        palette,
    );

    let appearance = card(
        column![
            settings_row(
                palette,
                Icon::Palette,
                "Sync Colors With System",
                None,
                app.switch_el(Flag::ThemeSync, true, palette),
                Some(InfoTip::plain(
                    "Takes the light/dark scheme and the accent colour from the GTK theme of the desktop.",
                )),
            ),
            divider(palette),
            settings_row(
                palette,
                Icon::Layers,
                "Sync Background & Elements",
                None,
                app.switch_el(Flag::SystemColors, true, palette),
                Some(InfoTip::plain(
                    "Also takes the window background, cards and status colours from the GTK theme.",
                )),
            ),
            divider(palette),
            settings_row(
                palette,
                Icon::Moon,
                "Theme",
                None,
                row![
                    text(if app.snapshot.theme_dark {
                        "Dark"
                    } else {
                        "Light"
                    })
                    .size(11.5)
                    .color(palette.muted),
                    app.switch_el(Flag::ThemeDark, !app.snapshot.theme_sync, palette),
                ]
                .spacing(8.0)
                .align_y(Vertical::Center)
                .into(),
                app.snapshot.theme_sync.then(|| {
                    InfoTip::plain("Turn off Sync Colors With System to choose the theme manually.")
                }),
            ),
        ]
        .width(Fill),
        palette,
    );

    let desktop_subtitle = match &app.desktop_status {
        DesktopIntegrationStatus::Aur { .. } => Some("Managed by system package manager (AUR)"),
        DesktopIntegrationStatus::Installed { .. } => {
            Some("Installed in ~/.local/share/applications and ~/.local/bin")
        }
        DesktopIntegrationStatus::NotInstalled => Some("Not currently added to applications menu"),
    };

    let config_card = card(
        column![
            settings_row(
                palette,
                Icon::File,
                "Export Rules",
                None,
                button(text("Export").size(11.0))
                    .padding(Padding {
                        top: 4.0,
                        bottom: 4.0,
                        left: 10.0,
                        right: 10.0,
                    })
                    .style(styled(ButtonKind::Subtle, palette))
                    .on_press(Message::ExportConfig)
                    .into(),
                None,
            ),
            divider(palette),
            settings_row(
                palette,
                Icon::File,
                "Import Rules",
                None,
                button(text("Import").size(11.0))
                    .padding(Padding {
                        top: 4.0,
                        bottom: 4.0,
                        left: 10.0,
                        right: 10.0,
                    })
                    .style(styled(ButtonKind::Subtle, palette))
                    .on_press(Message::ImportConfig)
                    .into(),
                None,
            ),
            divider(palette),
            settings_row(
                palette,
                Icon::Layers,
                "Desktop Integration",
                desktop_subtitle,
                match &app.desktop_status {
                    DesktopIntegrationStatus::Aur { .. } =>
                        button(text("How to Uninstall").size(11.0))
                            .padding(Padding {
                                top: 4.0,
                                bottom: 4.0,
                                left: 10.0,
                                right: 10.0,
                            })
                            .style(styled(ButtonKind::Subtle, palette))
                            .on_press(Message::ShowAurUninstallModal)
                            .into(),
                    DesktopIntegrationStatus::Installed { .. } => row![
                        button(text("Reinstall").size(11.0))
                            .padding(Padding {
                                top: 4.0,
                                bottom: 4.0,
                                left: 10.0,
                                right: 10.0,
                            })
                            .style(styled(ButtonKind::Subtle, palette))
                            .on_press(Message::InstallDesktopEntry),
                        button(text("Remove").size(11.0))
                            .padding(Padding {
                                top: 4.0,
                                bottom: 4.0,
                                left: 10.0,
                                right: 10.0,
                            })
                            .style(styled(ButtonKind::Subtle, palette))
                            .on_press(Message::UninstallDesktopEntry),
                    ]
                    .spacing(6.0)
                    .align_y(Vertical::Center)
                    .into(),
                    DesktopIntegrationStatus::NotInstalled =>
                        button(text("Add to System").size(11.0))
                            .padding(Padding {
                                top: 4.0,
                                bottom: 4.0,
                                left: 10.0,
                                right: 10.0,
                            })
                            .style(styled(ButtonKind::Primary, palette))
                            .on_press(Message::InstallDesktopEntry)
                            .into(),
                },
                None,
            ),
            divider(palette),
            settings_row(
                palette,
                Icon::Warning,
                "Purge All Data & Traces",
                Some("Deletes config files, systemd service, and desktop launchers"),
                button(text("Purge & Exit").size(11.0).color(palette.danger))
                    .padding(Padding {
                        top: 4.0,
                        bottom: 4.0,
                        left: 10.0,
                        right: 10.0,
                    })
                    .style(styled(ButtonKind::Subtle, palette))
                    .on_press(Message::ShowPurgeModal)
                    .into(),
                None,
            ),
        ]
        .width(Fill),
        palette,
    );

    column![
        diag_header,
        card(diag_list, palette),
        settings_card,
        appearance,
        config_card,
    ]
    .spacing(10.0)
    .width(Fill)
    .into()
}

fn diagnostics_row<'a>(entry: &'a DiagRow, palette: Palette) -> Element<'a, Message> {
    let (label, color) = match entry.status {
        DiagStatus::Ok => ("Ready", palette.success),
        DiagStatus::Warning => ("Unavailable", palette.warning),
        DiagStatus::Error => ("Error", palette.danger),
    };
    let label = entry.label.as_deref().unwrap_or(label);

    let mut control = row![].spacing(8.0).align_y(Vertical::Center);
    if let Some(value) = entry.inline_detail() {
        control = control.push(text(value).size(11.5).color(palette.muted));
    }
    control = control.push(text(label).size(12.0).color(color));

    if let Some(action) = entry.action {
        let message = match action {
            DiagAction::CheckVersion => Message::CheckVersion,
            DiagAction::OpenReleases => Message::OpenUrl(RELEASES_URL),
            DiagAction::InstallDesktop | DiagAction::UpdateDesktop => Message::InstallDesktopEntry,
            DiagAction::UninstallDesktop => Message::UninstallDesktopEntry,
            DiagAction::ShowAurUpdate => Message::ShowAurUpdateModal,
            DiagAction::ShowAurUninstall => Message::ShowAurUninstallModal,
            DiagAction::FixPath => Message::FixPath,
        };
        let btn_label = match action {
            DiagAction::CheckVersion => "Check",
            DiagAction::OpenReleases => "View Release",
            DiagAction::InstallDesktop => "Add to System",
            DiagAction::UpdateDesktop => "Update",
            DiagAction::UninstallDesktop => "Remove",
            DiagAction::ShowAurUpdate => "How to Update",
            DiagAction::ShowAurUninstall => "Uninstall Info",
            DiagAction::FixPath => "Fix PATH",
        };
        control = control.push(
            button(text(btn_label).size(11.0))
                .padding(Padding {
                    top: 4.0,
                    bottom: 4.0,
                    left: 10.0,
                    right: 10.0,
                })
                .style(styled(ButtonKind::Subtle, palette))
                .on_press_maybe((!entry.action_disabled).then_some(message)),
        );
    }

    settings_row(
        palette,
        entry.icon,
        &entry.title,
        entry.subtitle(),
        control.into(),
        None,
    )
}

fn bottom_bar(app: &App, palette: Palette) -> Element<'_, Message> {
    let (label, action, is_primary) = match app.page {
        Page::Rules => ("Run All", Message::RunAllRules, true),
        Page::Settings | Page::Logs(_) => ("Back", Message::BackToRules, false),
    };

    let btn = button(
        container(text(label).size(14.0).font(SEMIBOLD))
            .width(Fill)
            .height(Fill)
            .center_x(Fill)
            .center_y(Fill),
    )
    .width(Fill)
    .height(42.0)
    .padding(0.0)
    .on_press(action)
    .style(styled(
        if is_primary {
            ButtonKind::Primary
        } else {
            ButtonKind::Subtle
        },
        palette,
    ));

    container(btn)
        .width(Fill)
        .padding(Padding {
            top: 14.0,
            bottom: 16.0,
            left: 16.0,
            right: 16.0,
        })
        .into()
}

fn update_installed_modal(palette: Palette) -> Element<'static, Message> {
    let dialog = container(
        column![
            row![
                icon(Icon::Layers, 18.0, palette.accent),
                text("Update Installed Version?").size(14.0).font(BOLD),
            ]
            .spacing(10.0)
            .align_y(Vertical::Center),
            text(
                "A different version is currently installed in ~/.local/bin/totray.\n\nWould you like to replace the installed version with this current build and keep your desktop launcher updated?"
            )
            .size(12.0)
            .color(palette.muted),
            row![
                button(
                    container(text("Not Now").size(12.0))
                        .width(Fill)
                        .align_x(Horizontal::Center)
                )
                .padding(Padding {
                    top: 8.0,
                    bottom: 8.0,
                    left: 14.0,
                    right: 14.0,
                })
                .width(Length::Fixed(100.0))
                .style(styled(ButtonKind::Subtle, palette))
                .on_press(Message::DismissUpdateInstalledModal),
                button(
                    container(text("Replace Installed").size(12.0).font(SEMIBOLD))
                        .width(Fill)
                        .align_x(Horizontal::Center)
                )
                .padding(Padding {
                    top: 8.0,
                    bottom: 8.0,
                    left: 14.0,
                    right: 14.0,
                })
                .width(Fill)
                .style(styled(ButtonKind::Primary, palette))
                .on_press(Message::InstallDesktopEntry),
            ]
            .spacing(10.0)
            .width(Fill),
        ]
        .spacing(14.0)
        .width(360.0),
    )
    .padding(20.0)
    .style(move |_theme| container::Style {
        background: Some(Background::Color(palette.card)),
        border: Border {
            color: palette.hairline,
            width: 1.0,
            radius: 16.0.into(),
        },
        shadow: Shadow {
            color: alpha(Color::BLACK, 0.40),
            blur_radius: 24.0,
            ..Shadow::default()
        },
        ..container::Style::default()
    });

    container(dialog)
        .width(Fill)
        .height(Fill)
        .align_x(Horizontal::Center)
        .align_y(Vertical::Center)
        .style(move |_theme| container::Style {
            background: Some(Background::Color(alpha(Color::BLACK, 0.55))),
            ..container::Style::default()
        })
        .into()
}

fn aur_update_modal(
    package: &str,
    helper: &str,
    latest_version: &str,
    palette: Palette,
) -> Element<'static, Message> {
    let update_cmd = format!("{helper} -S {package}");
    let version_note = if latest_version.is_empty() {
        "A newer version is available.".to_string()
    } else {
        format!("A newer version (v{latest_version}) is available.")
    };
    let dialog = container(
        column![
            row![
                icon(Icon::Info, 18.0, palette.accent),
                text("Update via AUR").size(14.0).font(BOLD),
            ]
            .spacing(10.0)
            .align_y(Vertical::Center),
            text(format!(
                "{version_note}\n\nSince ToTray is managed as an AUR package ({package}), update it through your package manager by running:\n\n  {update_cmd}"
            ))
            .size(12.0)
            .color(palette.muted),
            button(
                container(text("Close").size(12.5).font(SEMIBOLD))
                    .width(Fill)
                    .align_x(Horizontal::Center),
            )
            .padding(Padding {
                top: 8.0,
                bottom: 8.0,
                left: 16.0,
                right: 16.0,
            })
            .width(Fill)
            .style(styled(ButtonKind::Primary, palette))
            .on_press(Message::DismissAurUpdateModal),
        ]
        .spacing(14.0)
        .width(360.0),
    )
    .padding(20.0)
    .style(move |_theme| container::Style {
        background: Some(Background::Color(palette.card)),
        border: Border {
            color: palette.hairline,
            width: 1.0,
            radius: 16.0.into(),
        },
        shadow: Shadow {
            color: alpha(Color::BLACK, 0.40),
            blur_radius: 24.0,
            ..Shadow::default()
        },
        ..container::Style::default()
    });

    container(dialog)
        .width(Fill)
        .height(Fill)
        .align_x(Horizontal::Center)
        .align_y(Vertical::Center)
        .style(move |_theme| container::Style {
            background: Some(Background::Color(alpha(Color::BLACK, 0.55))),
            ..container::Style::default()
        })
        .into()
}

fn aur_uninstall_modal(package: &str, helper: &str, palette: Palette) -> Element<'static, Message> {
    let uninstall_cmd = format!("{helper} -R {package}");
    let dialog = container(
        column![
            row![
                icon(Icon::Warning, 18.0, palette.warning),
                text("Uninstall AUR Package").size(14.0).font(BOLD),
            ]
            .spacing(10.0)
            .align_y(Vertical::Center),
            text(format!(
                "ToTray is installed as an AUR system package ({package}).\n\nTo uninstall the application and remove all system files, run this command in your terminal:\n\n  {uninstall_cmd}"
            ))
            .size(12.0)
            .color(palette.muted),
            button(
                container(text("Close").size(12.5).font(SEMIBOLD))
                    .width(Fill)
                    .align_x(Horizontal::Center),
            )
            .padding(Padding {
                top: 8.0,
                bottom: 8.0,
                left: 16.0,
                right: 16.0,
            })
            .width(Fill)
            .style(styled(ButtonKind::Primary, palette))
            .on_press(Message::DismissAurUninstallModal),
        ]
        .spacing(14.0)
        .width(360.0),
    )
    .padding(20.0)
    .style(move |_theme| container::Style {
        background: Some(Background::Color(palette.card)),
        border: Border {
            color: palette.hairline,
            width: 1.0,
            radius: 16.0.into(),
        },
        shadow: Shadow {
            color: alpha(Color::BLACK, 0.40),
            blur_radius: 24.0,
            ..Shadow::default()
        },
        ..container::Style::default()
    });

    container(dialog)
        .width(Fill)
        .height(Fill)
        .align_x(Horizontal::Center)
        .align_y(Vertical::Center)
        .style(move |_theme| container::Style {
            background: Some(Background::Color(alpha(Color::BLACK, 0.55))),
            ..container::Style::default()
        })
        .into()
}

fn purge_modal(
    is_aur: bool,
    aur_pkg: Option<&str>,
    aur_helper: Option<&str>,
    palette: Palette,
) -> Element<'static, Message> {
    let aur_note = if is_aur {
        let pkg = aur_pkg.unwrap_or("totray");
        let helper = aur_helper.unwrap_or("yay");
        format!(
            "\n\nNote: The application is installed via AUR ({pkg}). To completely remove the application binary from your system, run after exiting:\n  {helper} -R {pkg}"
        )
    } else {
        String::new()
    };

    let message_text = format!(
        "This will permanently delete:\n• All configuration files (~/.config/totray)\n• Systemd user service (totray.service)\n• Desktop launcher, icons, and ~/.local/bin/totray\n\nThe application will close immediately after cleanup.{aur_note}"
    );

    let dialog = container(
        column![
            row![
                icon(Icon::Warning, 18.0, palette.danger),
                text("Purge All Data & Exit?").size(14.0).font(BOLD),
            ]
            .spacing(10.0)
            .align_y(Vertical::Center),
            text(message_text).size(12.0).color(palette.muted),
            row![
                button(
                    container(text("Cancel").size(12.0))
                        .width(Fill)
                        .align_x(Horizontal::Center)
                )
                .padding(Padding {
                    top: 8.0,
                    bottom: 8.0,
                    left: 14.0,
                    right: 14.0,
                })
                .width(Length::Fixed(100.0))
                .style(styled(ButtonKind::Subtle, palette))
                .on_press(Message::DismissPurgeModal),
                button(
                    container(
                        text("Purge & Exit")
                            .size(12.0)
                            .font(SEMIBOLD)
                            .color(palette.danger),
                    )
                    .width(Fill)
                    .align_x(Horizontal::Center)
                )
                .padding(Padding {
                    top: 8.0,
                    bottom: 8.0,
                    left: 14.0,
                    right: 14.0,
                })
                .width(Fill)
                .style(styled(ButtonKind::Subtle, palette))
                .on_press(Message::ConfirmPurgeAndExit),
            ]
            .spacing(10.0)
            .width(Fill),
        ]
        .spacing(14.0)
        .width(380.0),
    )
    .padding(20.0)
    .style(move |_theme| container::Style {
        background: Some(Background::Color(palette.card)),
        border: Border {
            color: palette.hairline,
            width: 1.0,
            radius: 16.0.into(),
        },
        shadow: Shadow {
            color: alpha(Color::BLACK, 0.40),
            blur_radius: 24.0,
            ..Shadow::default()
        },
        ..container::Style::default()
    });

    container(dialog)
        .width(Fill)
        .height(Fill)
        .align_x(Horizontal::Center)
        .align_y(Vertical::Center)
        .style(move |_theme| container::Style {
            background: Some(Background::Color(alpha(Color::BLACK, 0.55))),
            ..container::Style::default()
        })
        .into()
}

fn card<'a>(content: impl Into<Element<'a, Message>>, palette: Palette) -> Element<'a, Message> {
    container(content)
        .width(Fill)
        .padding(Padding {
            top: 4.0,
            bottom: 4.0,
            left: 8.0,
            right: 8.0,
        })
        .style(move |_theme| container::Style {
            background: Some(Background::Color(palette.card)),
            border: Border {
                color: palette.hairline,
                width: 1.0,
                radius: 14.0.into(),
            },
            shadow: Shadow {
                color: alpha(Color::BLACK, 0.10),
                blur_radius: 12.0,
                ..Shadow::default()
            },
            ..container::Style::default()
        })
        .into()
}

fn divider<'a>(palette: Palette) -> Element<'a, Message> {
    container(Space::new().width(Fill).height(1.0))
        .width(Fill)
        .height(1.0)
        .style(move |_theme| container::Style {
            background: Some(Background::Color(palette.hairline)),
            ..container::Style::default()
        })
        .into()
}

#[derive(Clone, Copy)]
struct InfoTip<'a> {
    icon: Icon,
    text: &'a str,
}

impl<'a> InfoTip<'a> {
    fn plain(text: &'a str) -> Self {
        Self {
            icon: Icon::Info,
            text,
        }
    }
}

fn settings_row<'a>(
    palette: Palette,
    icon_kind: Icon,
    title: &'a str,
    subtitle: Option<&'a str>,
    control: Element<'a, Message>,
    info: Option<InfoTip<'a>>,
) -> Element<'a, Message> {
    row_with_meta(palette, icon_kind, title, subtitle, control, info, false)
}

fn settings_sub_row<'a>(
    palette: Palette,
    icon_kind: Icon,
    title: &'a str,
    subtitle: Option<&'a str>,
    control: Element<'a, Message>,
    info: Option<InfoTip<'a>>,
) -> Element<'a, Message> {
    row_with_meta(palette, icon_kind, title, subtitle, control, info, true)
}

fn icon_tile(kind: Icon, palette: Palette) -> Element<'static, Message> {
    container(icon(kind, 15.0, palette.muted))
        .width(Length::Fixed(28.0))
        .height(Length::Fixed(28.0))
        .center_x(Length::Fixed(28.0))
        .center_y(Length::Fixed(28.0))
        .style(move |_theme| container::Style {
            background: Some(Background::Color(palette.surface_hover)),
            border: Border {
                color: Color::TRANSPARENT,
                width: 0.0,
                radius: 9.0.into(),
            },
            ..container::Style::default()
        })
        .into()
}

#[allow(clippy::too_many_arguments)]
fn row_with_meta<'a>(
    palette: Palette,
    icon_kind: Icon,
    title: &'a str,
    subtitle: Option<&'a str>,
    control: Element<'a, Message>,
    info: Option<InfoTip<'a>>,
    indent: bool,
) -> Element<'a, Message> {
    let mut labels = column![text(title).size(13.0)].width(Fill).spacing(2.0);
    if let Some(subtitle) = subtitle {
        labels = labels.push(text(subtitle).size(11.0).color(palette.muted));
    }

    let mut right = row![].spacing(6.0).align_y(Vertical::Center);
    if let Some(info) = info {
        right = right.push(tip(
            icon(info.icon, 14.0, palette.faint),
            container(text(info.text).size(11.0).width(Length::Fixed(220.0))).padding(2.0),
            TooltipPosition::Left,
            palette,
        ));
    }
    right = right.push(control);

    let tile = icon_tile(icon_kind, palette);

    let mut content = row![].spacing(10.0).align_y(Vertical::Center).width(Fill);
    if indent {
        content = content.push(Space::new().width(Length::Fixed(14.0)));
    }
    content = content.push(tile).push(labels).push(right);

    container(content)
        .width(Fill)
        .padding(Padding {
            top: 5.0,
            bottom: 5.0,
            left: 0.0,
            right: 0.0,
        })
        .into()
}

fn switch<'a>(
    is_on: bool,
    enabled: bool,
    progress: f32,
    palette: Palette,
    on_toggle: impl Fn(bool) -> Message + 'a,
) -> Element<'a, Message> {
    let progress = progress.clamp(0.0, 1.0);
    let track_color = if enabled {
        mix(alpha(palette.fg, 0.22), palette.accent, progress)
    } else {
        alpha(palette.fg, 0.12)
    };

    let knob =
        container(Space::new().width(16.0).height(16.0)).style(move |_theme| container::Style {
            background: Some(Background::Color(if enabled {
                Color::WHITE
            } else {
                alpha(Color::WHITE, 0.65)
            })),
            border: Border {
                color: Color::TRANSPARENT,
                width: 0.0,
                radius: 8.0.into(),
            },
            ..container::Style::default()
        });

    let track = container(
        row![
            Space::new().width(Length::Fixed(3.0 + progress * 18.0)),
            knob,
            Space::new().width(Fill),
        ]
        .height(22.0)
        .align_y(Vertical::Center),
    )
    .width(40.0)
    .height(22.0)
    .style(move |_theme| container::Style {
        background: Some(Background::Color(track_color)),
        border: Border {
            color: Color::TRANSPARENT,
            width: 0.0,
            radius: 11.0.into(),
        },
        ..container::Style::default()
    });

    button(track)
        .padding(0.0)
        .style(switch_button_style(palette))
        .on_press_maybe(enabled.then(|| on_toggle(!is_on)))
        .into()
}

fn switch_button_style(palette: Palette) -> impl Fn(&Theme, button::Status) -> button::Style {
    move |_theme, status| {
        let background = match status {
            button::Status::Hovered => Some(Background::Color(alpha(palette.accent, 0.16))),
            _ => None,
        };
        button::Style {
            background,
            text_color: palette.fg,
            border: Border {
                color: Color::TRANSPARENT,
                width: 3.0,
                radius: 14.0.into(),
            },
            shadow: Shadow::default(),
            ..button::Style::default()
        }
    }
}

fn transparent_button_style(palette: Palette) -> impl Fn(&Theme, button::Status) -> button::Style {
    move |_theme, status| {
        let text_color = match status {
            button::Status::Hovered => palette.accent,
            button::Status::Pressed => alpha(palette.accent, 0.8),
            _ => palette.fg,
        };
        button::Style {
            background: None,
            text_color,
            border: Border::default(),
            shadow: Shadow::default(),
            ..button::Style::default()
        }
    }
}

fn icon_button(kind: Icon, palette: Palette, message: Message) -> Element<'static, Message> {
    button(
        container(icon(kind, 14.0, palette.muted))
            .width(Length::Fixed(26.0))
            .height(Length::Fixed(26.0))
            .center_x(Length::Fixed(26.0))
            .center_y(Length::Fixed(26.0)),
    )
    .width(26.0)
    .height(26.0)
    .padding(0.0)
    .on_press(message)
    .style(header_button_style(palette))
    .into()
}

fn header_button_style(palette: Palette) -> impl Fn(&Theme, button::Status) -> button::Style {
    move |_theme, status| {
        let background = match status {
            button::Status::Hovered | button::Status::Pressed => {
                Some(Background::Color(palette.surface_hover))
            }
            _ => None,
        };
        button::Style {
            background,
            text_color: palette.fg,
            border: Border {
                color: palette.border,
                width: 1.0,
                radius: 7.0.into(),
            },
            shadow: Shadow::default(),
            ..button::Style::default()
        }
    }
}

#[derive(Clone, Copy)]
enum ButtonKind {
    Primary,
    Subtle,
}

fn style_for(kind: ButtonKind) -> fn(Palette, &Theme, button::Status) -> button::Style {
    match kind {
        ButtonKind::Primary => primary_style,
        ButtonKind::Subtle => subtle_style,
    }
}

fn styled(kind: ButtonKind, palette: Palette) -> impl Fn(&Theme, button::Status) -> button::Style {
    move |theme, status| style_for(kind)(palette, theme, status)
}

fn base_button(background: Color, text_color: Color, radius: f32) -> button::Style {
    button::Style {
        background: Some(Background::Color(background)),
        text_color,
        border: Border {
            color: Color::TRANSPARENT,
            width: 0.0,
            radius: radius.into(),
        },
        shadow: Shadow::default(),
        ..button::Style::default()
    }
}

fn primary_style(palette: Palette, _theme: &Theme, status: button::Status) -> button::Style {
    match status {
        button::Status::Hovered => base_button(
            mix(palette.accent, Color::WHITE, 0.12),
            palette.on_accent,
            10.0,
        ),
        button::Status::Pressed => base_button(
            mix(palette.accent, Color::BLACK, 0.12),
            palette.on_accent,
            10.0,
        ),
        button::Status::Disabled => base_button(
            alpha(palette.accent, 0.4),
            alpha(palette.on_accent, 0.6),
            10.0,
        ),
        _ => base_button(palette.accent, palette.on_accent, 10.0),
    }
}

fn subtle_style(palette: Palette, _theme: &Theme, status: button::Status) -> button::Style {
    let background = match status {
        button::Status::Hovered | button::Status::Pressed => palette.surface_hover,
        _ => palette.surface,
    };
    let text_color = match status {
        button::Status::Disabled => palette.faint,
        _ => palette.fg,
    };
    button::Style {
        background: Some(Background::Color(background)),
        text_color,
        border: Border {
            color: palette.border,
            width: 1.0,
            radius: 9.0.into(),
        },
        shadow: Shadow::default(),
        ..button::Style::default()
    }
}

fn build_diagnostics(
    version: &VersionState,
    desktop_status: &DesktopIntegrationStatus,
    path_ok: bool,
) -> Vec<DiagRow> {
    let mut rows = Vec::new();

    let (version_label, version_status, version_detail, version_action) = if version.checking {
        (
            Some("Checking...".to_string()),
            DiagStatus::Ok,
            None,
            Some(DiagAction::CheckVersion),
        )
    } else {
        match version.result.as_ref() {
            Some(Ok((true, _))) => (
                Some(format!("v{CURRENT_VERSION} - latest")),
                DiagStatus::Ok,
                None,
                Some(DiagAction::CheckVersion),
            ),
            Some(Ok((false, latest))) => {
                if let DesktopIntegrationStatus::Aur { package, helper } = desktop_status {
                    (
                        Some(format!("v{latest} available (AUR)")),
                        DiagStatus::Warning,
                        Some(format!("Update via AUR: {helper} -S {package}")),
                        Some(DiagAction::ShowAurUpdate),
                    )
                } else {
                    (
                        Some(format!("v{latest} available")),
                        DiagStatus::Warning,
                        Some("New release on GitHub".to_string()),
                        Some(DiagAction::OpenReleases),
                    )
                }
            }
            Some(Err(_)) => (
                Some("Unavailable".to_string()),
                DiagStatus::Warning,
                None,
                Some(DiagAction::CheckVersion),
            ),
            None => (None, DiagStatus::Ok, None, Some(DiagAction::CheckVersion)),
        }
    };

    rows.push(DiagRow {
        icon: Icon::Info,
        title: "Version".to_string(),
        detail: version_detail,
        status: version_status,
        action: version_action,
        action_disabled: version.checking,
        label: version_label,
    });

    let hypr_ok = crate::backend::is_hyprland();
    rows.push(DiagRow {
        icon: if hypr_ok { Icon::Check } else { Icon::Warning },
        title: "Hyprland Environment".to_string(),
        detail: Some(if hypr_ok {
            "HYPRLAND_INSTANCE_SIGNATURE active".to_string()
        } else {
            "ToTray requires Hyprland to manage windows".to_string()
        }),
        status: if hypr_ok {
            DiagStatus::Ok
        } else {
            DiagStatus::Error
        },
        action: None,
        action_disabled: false,
        label: None,
    });

    let (desktop_detail, desktop_status_type, desktop_label, desktop_action) = match desktop_status
    {
        DesktopIntegrationStatus::Aur { package, .. } => (
            Some(format!("Managed by AUR package ({package})")),
            DiagStatus::Ok,
            Some("AUR Package".to_string()),
            Some(DiagAction::ShowAurUninstall),
        ),
        DesktopIntegrationStatus::Installed { up_to_date: true } => (
            Some("Desktop launcher and icon active (~/.local/bin)".to_string()),
            DiagStatus::Ok,
            Some("Installed".to_string()),
            Some(DiagAction::UninstallDesktop),
        ),
        DesktopIntegrationStatus::Installed { up_to_date: false } => (
            Some("Installed binary in ~/.local/bin differs from this build".to_string()),
            DiagStatus::Warning,
            Some("Update Available".to_string()),
            Some(DiagAction::UpdateDesktop),
        ),
        DesktopIntegrationStatus::NotInstalled => (
            Some("Add to applications menu and desktop launcher".to_string()),
            DiagStatus::Warning,
            Some("Not Installed".to_string()),
            Some(DiagAction::InstallDesktop),
        ),
    };

    rows.push(DiagRow {
        icon: Icon::Layers,
        title: "Desktop Integration".to_string(),
        detail: desktop_detail,
        status: desktop_status_type,
        action: desktop_action,
        action_disabled: false,
        label: desktop_label,
    });

    rows.push(DiagRow {
        icon: if path_ok { Icon::Check } else { Icon::Warning },
        title: "Terminal PATH".to_string(),
        detail: Some(if path_ok {
            "~/.local/bin is in PATH".to_string()
        } else {
            "~/.local/bin is not in PATH".to_string()
        }),
        status: if path_ok {
            DiagStatus::Ok
        } else {
            DiagStatus::Warning
        },
        action: (!path_ok).then_some(DiagAction::FixPath),
        action_disabled: false,
        label: Some(if path_ok {
            "Configured".to_string()
        } else {
            "Missing".to_string()
        }),
    });

    rows
}

async fn check_latest_version() -> Result<(bool, String), String> {
    let output = Command::new("curl")
        .args([
            "-s",
            "--fail",
            "--connect-timeout",
            "3",
            "https://raw.githubusercontent.com/agzes/ToTray/main/version",
        ])
        .output();

    if let Ok(output) = output
        && output.status.success()
    {
        let latest = String::from_utf8_lossy(&output.stdout).trim().to_string();
        if !latest.is_empty() && !latest.contains("Not Found") {
            return Ok((CURRENT_VERSION == latest, latest));
        }
    }

    Err("Failed to check for updates".to_string())
}

async fn pick_save_file() -> Option<PathBuf> {
    if let Ok(out) = Command::new("zenity")
        .args([
            "--file-selection",
            "--save",
            "--confirm-overwrite",
            "--title=Export Configuration",
            "--filename=totray_config.json",
        ])
        .output()
        && out.status.success()
    {
        let s = String::from_utf8_lossy(&out.stdout).trim().to_string();
        if !s.is_empty() {
            return Some(PathBuf::from(s));
        }
    }

    if let Ok(out) = Command::new("kdialog")
        .args([
            "--getsavefilename",
            "totray_config.json",
            "*.json|JSON files",
        ])
        .output()
        && out.status.success()
    {
        let s = String::from_utf8_lossy(&out.stdout).trim().to_string();
        if !s.is_empty() {
            return Some(PathBuf::from(s));
        }
    }

    None
}

async fn pick_open_file() -> Option<PathBuf> {
    if let Ok(out) = Command::new("zenity")
        .args([
            "--file-selection",
            "--title=Import Configuration",
            "--file-filter=JSON files | *.json",
        ])
        .output()
        && out.status.success()
    {
        let s = String::from_utf8_lossy(&out.stdout).trim().to_string();
        if !s.is_empty() {
            return Some(PathBuf::from(s));
        }
    }

    if let Ok(out) = Command::new("kdialog")
        .args(["--getopenfilename", ".", "*.json|JSON files"])
        .output()
        && out.status.success()
    {
        let s = String::from_utf8_lossy(&out.stdout).trim().to_string();
        if !s.is_empty() {
            return Some(PathBuf::from(s));
        }
    }

    None
}

fn system_dark_preference() -> Option<bool> {
    if let Ok(theme) = std::env::var("GTK_THEME") {
        return Some(theme.to_lowercase().contains("dark"));
    }

    let mut answered = false;
    for (schema, key) in [
        ("org.gnome.desktop.interface", "color-scheme"),
        ("org.gnome.desktop.interface", "gtk-theme"),
        ("org.kde.gtk.config", "darkMode"),
    ] {
        let Some(value) = gsettings_value(schema, key) else {
            continue;
        };
        answered = true;
        let value = value.to_lowercase();
        if value.contains("prefer-dark") || value.contains("dark") || value.trim() == "true" {
            return Some(true);
        }
    }

    answered.then_some(false)
}

fn gsettings_value(schema: &str, key: &str) -> Option<String> {
    let output = Command::new("gsettings")
        .args(["get", schema, key])
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    Some(String::from_utf8_lossy(&output.stdout).to_string())
}

#[derive(Clone, Copy)]
struct SystemColors {
    background: Color,
    text: Color,
    card: Color,
    success: Color,
    danger: Color,
}

fn system_stylesheets() -> Vec<String> {
    let home = std::env::var("HOME").unwrap_or_default();
    let mut files = vec![
        format!("{home}/.config/gtk-4.0/gtk.css"),
        format!("{home}/.config/gtk-3.0/gtk.css"),
    ];

    if let Some(theme) = gsettings_value("org.gnome.desktop.interface", "gtk-theme") {
        let theme = theme.trim().trim_matches('\'');
        if !theme.is_empty() && !theme.contains(char::is_whitespace) {
            let mut roots = vec![
                format!("{home}/.themes"),
                format!("{home}/.local/share/themes"),
            ];
            let data_dirs = std::env::var("XDG_DATA_DIRS")
                .unwrap_or_else(|_| "/usr/local/share:/usr/share".into());
            for root in data_dirs.split(':').filter(|root| !root.is_empty()) {
                roots.push(format!("{root}/themes"));
            }
            for root in roots {
                for css in ["gtk-4.0/gtk.css", "gtk-4.0/colors.css", "gtk-3.0/gtk.css"] {
                    files.push(format!("{root}/{theme}/{css}"));
                }
            }
        }
    }

    files
}

fn system_accent(dark: bool) -> Option<Color> {
    for path in system_stylesheets() {
        if let Ok(text) = std::fs::read_to_string(&path)
            && let Some(color) = gtk_accent(&text, dark)
        {
            return Some(color);
        }
    }

    None
}

fn system_palette(dark: bool) -> Option<SystemColors> {
    for path in system_stylesheets() {
        if let Ok(text) = std::fs::read_to_string(&path)
            && let Some(colors) = parse_system_palette(&text, dark)
        {
            return Some(colors);
        }
    }

    None
}

fn parse_system_palette(css: &str, dark: bool) -> Option<SystemColors> {
    let definitions = gtk_definitions(css);
    let value = |key: &str| {
        lookup_definition(&definitions, key, dark)
            .and_then(|value| resolve_value(value, &definitions, dark, 0))
    };

    let background = value("window_bg_color")?;
    let text = value("window_fg_color")?;
    Some(SystemColors {
        background,
        text,
        card: value("card_bg_color").unwrap_or_else(|| mix(background, text, 0.07)),
        success: value("success_color").unwrap_or(if dark { SUCCESS_DARK } else { SUCCESS_LIGHT }),
        danger: value("destructive_color").unwrap_or(if dark { DANGER_DARK } else { DANGER_LIGHT }),
    })
}

fn gtk_accent(css: &str, dark: bool) -> Option<Color> {
    let definitions = gtk_definitions(css);

    for key in [
        "accent_bg_color",
        "accent_color",
        "theme_selected_bg_color",
        "selected_bg_color",
    ] {
        let Some(value) = lookup_definition(&definitions, key, dark) else {
            continue;
        };
        if let Some(color) = resolve_value(value, &definitions, dark, 0) {
            return Some(color);
        }
    }

    None
}

fn gtk_definitions(css: &str) -> Vec<(Option<bool>, &str, &str)> {
    let mut definitions = Vec::new();
    let mut context: Option<bool> = None;
    let mut depth = 0usize;

    for line in css.lines() {
        let trimmed = line.trim();
        if let Some(rest) = trimmed.strip_prefix("@media") {
            context = if rest.contains("prefers-color-scheme: dark") {
                Some(true)
            } else if rest.contains("prefers-color-scheme: light") {
                Some(false)
            } else {
                None
            };
        }

        if let Some(rest) = trimmed.strip_prefix("@define-color")
            && rest.starts_with(char::is_whitespace)
        {
            let rest = rest.trim_start();
            if let Some((name, value)) = rest.split_once(char::is_whitespace) {
                definitions.push((context, name, value.trim().trim_end_matches(';')));
            }
        }

        depth += line.matches('{').count();
        let closes = line.matches('}').count();
        depth = depth.saturating_sub(closes);
        if closes > 0 && depth == 0 {
            context = None;
        }
    }

    definitions
}

fn lookup_definition<'a>(
    definitions: &[(Option<bool>, &'a str, &'a str)],
    name: &str,
    dark: bool,
) -> Option<&'a str> {
    for wanted in [Some(dark), None] {
        let mut found: Option<&str> = None;
        for (context, key, value) in definitions {
            if *key == name && *context == wanted {
                found = Some(*value);
            }
        }
        if found.is_some() {
            return found;
        }
    }
    None
}

fn resolve_value(
    value: &str,
    definitions: &[(Option<bool>, &str, &str)],
    dark: bool,
    depth: u8,
) -> Option<Color> {
    if depth > 5 {
        return None;
    }
    let value = value.trim();
    if let Some(hex) = value.strip_prefix('#') {
        return parse_hex_color(hex);
    }
    let reference = value.strip_prefix('@')?;
    let target = lookup_definition(definitions, reference, dark)?;
    resolve_value(target, definitions, dark, depth + 1)
}

fn parse_hex_color(hex: &str) -> Option<Color> {
    let component = |range: std::ops::Range<usize>| u8::from_str_radix(hex.get(range)?, 16).ok();
    match hex.len() {
        6 => Some(Color::from_rgb8(
            component(0..2)?,
            component(2..4)?,
            component(4..6)?,
        )),
        8 => Some(Color::from_rgba8(
            component(0..2)?,
            component(2..4)?,
            component(4..6)?,
            component(6..8)? as f32 / 255.0,
        )),
        _ => None,
    }
}
