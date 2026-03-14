use crate::state::{Action, AppRule, SharedState};
use gtk::gdk_pixbuf::PixbufLoader;
use gtk::prelude::*;
use gtk::{
    Adjustment, Align, Application, ApplicationWindow, Box as GBox, Button, CssProvider, DropDown,
    Entry, FileDialog, FileFilter, GestureClick, Image, Label, ListBox, ListBoxRow, ListItem,
    Orientation, Revealer, SignalListItemFactory, SpinButton, Stack, StringList, Switch,
};
use std::process::Command;

pub const CURRENT_VERSION: &str = "0.1.0";

const CSS: &str = "
    .main-window { background-color: @window_bg_color; }
    .main-box { padding: 20px; padding-bottom: 14px; }
    .header-box { margin-bottom: 20px; }
    .app-title { font-size: 24px; font-weight: 800; letter-spacing: -0.5px; }
    .version-btn {
        font-size: 9px;
        font-weight: bold;
        padding: 1px 5px;
        border-radius: 6px;
        background-color: alpha(@window_fg_color, 0.1);
        color: alpha(@window_fg_color, 0.6);
        margin-left: 8px;
        border: none;
        min-height: 18px;
    }
    .version-btn:hover {
        background-color: alpha(@window_fg_color, 0.2);
        color: @window_fg_color;
    }
    .icon-btn {
        padding: 0;
        min-width: 22px;
        min-height: 22px;
        border-radius: 6px;
    }
    .app-subtitle { font-size: 12px; margin-top: -2px; }
    .app-subtitle a, .badge-link a { color: inherit; text-decoration: none; font-weight: bold; }
    .app-subtitle a:hover { opacity: 1.0; text-decoration: underline; }
    .section-title { font-size: 10px; font-weight: bold; text-transform: uppercase; letter-spacing: 0.5px; opacity: 0.5; }
    .card { background-color: alpha(@window_fg_color, 0.03); border: 1px solid alpha(@window_fg_color, 0.08); border-radius: 12px; overflow: hidden; transition: all 300ms ease; }
    .card:hover { background-color: alpha(@window_fg_color, 0.05); border-color: alpha(@window_fg_color, 0.15); }
    list { background-color: transparent; }
    row { padding: 8px 14px; border-bottom: 1px solid alpha(@window_fg_color, 0.04); transition: background-color 200ms ease; }
    row:hover { background-color: alpha(@window_fg_color, 0.02); }
    row label.row-title { font-weight: 500; font-size: 14px; }
    row label.row-subtitle { font-size: 11px; opacity: 0.5; }
    row.sub-row > box { margin-left: 20px; opacity: 0.85; }
    row.sub-row label.row-title { font-size: 13px; }
    .info-icon { opacity: 0.4; }
    .info-icon:hover { opacity: 0.9; }
    .badge-link { padding: 2px 6px; border-radius: 6px; font-size: 9px; font-weight: 800; background-color: alpha(@window_fg_color, 0.06); color: alpha(@window_fg_color, 0.5); }
    .badge-link:hover { background-color: alpha(@window_fg_color, 0.12); color: @window_fg_color; }
    .beta-badge { font-size: 9px; font-weight: 800; padding: 1px 5px; border-radius: 5px; background-color: #f5c71a; color: #000; margin-left: 0px; }
    dropdown button { padding: 0 6px; min-height: 26px; font-size: 12px; border-radius: 6px; }
    spinbutton { min-height: 26px; font-size: 12px; border-radius: 6px; padding: 0; }
    spinbutton button { padding: 0 4px; min-height: 22px; }
    switch { margin: 0; transform: scale(0.85); }
    entry { min-height: 26px; font-size: 12px; border-radius: 6px; padding: 0 8px; border: 1px solid alpha(@window_fg_color, 0.1); background: alpha(@window_fg_color, 0.05); }
    .start-button, .stop-button { border-radius: 12px; padding: 12px; font-weight: 800; font-size: 14px; border: none; transition: all 300ms cubic-bezier(0.25, 1, 0.5, 1); margin-bottom: 10px; }
    .start-button { background-color: @accent_bg_color; color: @accent_fg_color; box-shadow: 0 4px 0px alpha(@accent_bg_color, 0.5); }
    .start-button:hover { background-color: alpha(@accent_bg_color, 0.9); transform: translateY(-2px); box-shadow: 0 6px 0px alpha(@accent_bg_color, 0.4); }
    .start-button:active { transform: translateY(2px); box-shadow: 0 2px 0px alpha(@accent_bg_color, 0.6); }
    .stop-button { background-color: #ff5555; color: white; box-shadow: 0 4px 0px #cc0000; }
    .stop-button:hover { background-color: #ff6666; transform: translateY(-2px); box-shadow: 0 6px 0px #cc0000; }
    .stop-button:active { transform: translateY(2px); box-shadow: 0 2px 0px #cc0000; }
    .status-badge { padding: 2px 8px; border-radius: 8px; font-size: 9px; font-weight: 800; text-align: center; }
    .status-badge.active { background-color: alpha(@accent_bg_color, 0.2); color: @accent_bg_color; }
    .status-badge.inactive { background-color: alpha(@window_fg_color, 0.1); color: alpha(@window_fg_color, 0.6); }

    .bottom-card {
        background: alpha(@window_fg_color, 0.04);
        border: 1px solid alpha(@window_fg_color, 0.08);
        border-radius: 16px;
        padding: 4px;
        margin-top: 10px;
    }
    .expander-header {
        padding: 12px 16px;
        border-radius: 12px;
        transition: all 250ms ease;
    }
    .expander-header:hover {
        background: alpha(@window_fg_color, 0.05);
    }
    .expander-title {
        font-weight: 800;
        font-size: 14px;
        opacity: 0.9;
    }
    .expander-icon {
        transition: all 300ms cubic-bezier(0.25, 1, 0.5, 1);
        opacity: 0.6;
    }
    .expander-icon.expanded {
        transform: rotate(180deg);
        opacity: 1;
        color: @accent_color;
    }
    .reveal-content {
        margin: 8px;
        margin-top: 10px;
    }

    .compat-item { padding: 12px; border-radius: 12px; background: alpha(@window_fg_color, 0.03); border: 1px solid alpha(@window_fg_color, 0.06); margin-bottom: 8px; transition: all 300ms ease; }
    .compat-item:hover { background-color: alpha(@window_fg_color, 0.05); }
    .compat-item.error { border-left: 4px solid #ff5555; }
    .compat-item.ok { border-left: 4px solid #8fde58; }
    .compat-item.warning-item { border-left: 4px solid #f5c71a; }
    .compat-item.info-item { border-left: 4px solid #3584e4; }
    .tutorial-text { font-size: 11px; opacity: 0.6; margin-top: 4px; }
    .compat-title { font-size: 16px; font-weight: 800; margin-bottom: 16px; opacity: 0.9; }
    .compat-name { font-size: 13px; font-weight: bold; }
    .welcome-title { font-size: 32px; font-weight: 800; letter-spacing: -1px; }
    .welcome-subtitle { font-size: 16px; margin-bottom: 20px; }
    .info-note {
        background-color: alpha(@window_fg_color, 0.03);
        border: 1px solid alpha(@window_fg_color, 0.07);
        padding: 20px;
        border-radius: 16px;
        margin: 10px 0;
        transition: transform 300ms ease;
    }
    .info-note:hover { transform: scale(1.02); }
";

fn create_row(
    title: &str,
    subtitle: Option<&str>,
    widget: &impl IsA<gtk::Widget>,
    info_text: Option<&str>,
) -> ListBoxRow {
    let row = ListBoxRow::new();
    let main_hbox = GBox::new(Orientation::Horizontal, 12);
    main_hbox.set_valign(Align::Center);
    main_hbox.set_hexpand(true);
    let text_vbox = GBox::new(Orientation::Vertical, 0);
    text_vbox.set_valign(Align::Center);
    let title_hbox = GBox::new(Orientation::Horizontal, 4);
    title_hbox.set_valign(Align::Center);
    if let Some(txt) = info_text {
        let info_img = Image::from_icon_name("info-symbolic");
        info_img.add_css_class("info-icon");
        info_img.set_tooltip_text(Some(txt));
        title_hbox.append(&info_img);
    }
    title_hbox.append(
        &Label::builder()
            .label(title)
            .halign(Align::Start)
            .css_classes(["row-title"])
            .wrap(true)
            .xalign(0.0)
            .build(),
    );
    text_vbox.append(&title_hbox);
    if let Some(sub) = subtitle {
        let sub_label = Label::builder()
            .label(sub)
            .halign(Align::Start)
            .css_classes(["row-subtitle"])
            .wrap(true)
            .xalign(0.0)
            .build();
        text_vbox.append(&sub_label);
    }
    main_hbox.append(&text_vbox);
    let filler = GBox::new(Orientation::Horizontal, 0);
    filler.set_hexpand(true);
    main_hbox.append(&filler);
    widget.set_valign(Align::Center);
    main_hbox.append(widget);
    row.set_child(Some(&main_hbox));
    row.set_activatable(false);
    row.set_selectable(false);
    row
}

fn create_rule_row(
    rule: &AppRule,
    state: SharedState,
    list: &ListBox,
    empty_row: &ListBoxRow,
) -> ListBoxRow {
    let row = ListBoxRow::new();
    row.set_activatable(false);
    row.set_selectable(false);
    row.set_can_focus(false);
    let main_hbox = GBox::new(Orientation::Horizontal, 8);
    main_hbox.set_valign(Align::Center);
    main_hbox.set_hexpand(true);

    let text_vbox = GBox::new(Orientation::Vertical, 0);
    text_vbox.set_valign(Align::Center);
    text_vbox.append(
        &Label::builder()
            .label(&rule.name)
            .halign(Align::Start)
            .css_classes(["row-title"])
            .ellipsize(gtk::pango::EllipsizeMode::End)
            .build(),
    );
    text_vbox.append(
        &Label::builder()
            .label(&rule.exec)
            .halign(Align::Start)
            .css_classes(["row-subtitle"])
            .ellipsize(gtk::pango::EllipsizeMode::End)
            .build(),
    );
    main_hbox.append(&text_vbox);

    let filler = GBox::new(Orientation::Horizontal, 0);
    filler.set_hexpand(true);
    main_hbox.append(&filler);

    let act_text = match rule.action {
        Action::Close => "Close",
        Action::Close2 => "Close 2x",
        Action::Workspace(n) => Box::leak(format!("WS {n}").into_boxed_str()),
        Action::HideToTray => "Tray",
    };
    main_hbox.append(
        &Label::builder()
            .label(act_text)
            .css_classes(["row-subtitle"])
            .margin_end(4)
            .build(),
    );

    let run_btn = Button::builder()
        .icon_name("media-playback-start-symbolic")
        .valign(Align::Center)
        .build();
    run_btn.add_css_class("icon-btn");
    run_btn.set_tooltip_text(Some("Run now"));
    let rule_run = rule.clone();
    let state_run = state.clone();
    run_btn.connect_clicked(move |_| {
        let (delay, notify) = {
            let s = state_run.lock().unwrap();
            (s.launch_delay, s.notifications)
        };
        crate::backend::run_rule(&rule_run, delay, notify, &state_run);
    });
    main_hbox.append(&run_btn);

    let log_btn = Button::builder()
        .icon_name("utilities-terminal-symbolic")
        .valign(Align::Center)
        .build();
    log_btn.add_css_class("icon-btn");
    log_btn.set_tooltip_text(Some("Open Console"));
    let name_log = rule.name.clone();
    let state_log = state.clone();
    log_btn.connect_clicked(move |_| {
        show_logs_window(&name_log, state_log.clone());
    });
    main_hbox.append(&log_btn);

    let del_btn = Button::builder()
        .icon_name("user-trash-symbolic")
        .valign(Align::Center)
        .build();
    del_btn.add_css_class("icon-btn");
    let name = rule.name.clone();
    let state_c = state.clone();
    let list_w = list.downgrade();
    let row_w = row.downgrade();
    let er_c = empty_row.clone();

    del_btn.connect_clicked(move |_| {
        let mut s = state_c.lock().unwrap();
        s.apps.retain(|r| r.name != name);
        s.save();
        if let (Some(l), Some(r)) = (list_w.upgrade(), row_w.upgrade()) {
            l.remove(&r);
            er_c.set_visible(s.apps.is_empty());
        }
    });
    main_hbox.append(&del_btn);

    row.set_child(Some(&main_hbox));
    row
}

fn show_logs_window(name: &str, state: SharedState) {
    let window = gtk::Window::builder()
        .title(format!("Console - {}", name))
        .default_width(500)
        .default_height(300)
        .build();

    let vbox = GBox::new(Orientation::Vertical, 8);
    vbox.set_margin_start(10);
    vbox.set_margin_end(10);
    vbox.set_margin_top(10);
    vbox.set_margin_bottom(10);

    let scrolled = gtk::ScrolledWindow::builder().vexpand(true).build();

    let text_view = gtk::TextView::builder()
        .editable(false)
        .cursor_visible(false)
        .monospace(true)
        .build();

    scrolled.set_child(Some(&text_view));
    vbox.append(&scrolled);

    let buffer = text_view.buffer();

    {
        let s = state.lock().unwrap();
        if let Some(logs) = s.logs.get(name) {
            let mut text = String::new();
            for line in logs {
                text.push_str(line);
                text.push('\n');
            }
            buffer.set_text(&text);
        } else {
            buffer.set_text("No logs available yet. Start the app to see output.");
        }
    }

    let state_c = state.clone();
    let name_c = name.to_string();
    let buffer_c = buffer.clone();
    let window_weak = window.downgrade();
    glib::timeout_add_local(std::time::Duration::from_millis(500), move || {
        let win = match window_weak.upgrade() {
            Some(w) => w,
            None => return glib::ControlFlow::Break,
        };

        if !win.is_visible() {
            return glib::ControlFlow::Break;
        }

        let s = state_c.lock().unwrap();
        if let Some(logs) = s.logs.get(&name_c) {
            let mut text = String::new();
            for line in logs {
                text.push_str(line);
                text.push('\n');
            }
            if buffer_c.text(&buffer_c.start_iter(), &buffer_c.end_iter(), false) != text {
                buffer_c.set_text(&text);
                let mark = buffer_c.create_mark(None, &buffer_c.end_iter(), false);
                text_view.scroll_to_mark(&mark, 0.0, true, 0.5, 1.0);
            }
        }
        glib::ControlFlow::Continue
    });

    window.set_child(Some(&vbox));
    window.present();
}

pub fn build_ui(app: &Application, state: SharedState) -> ApplicationWindow {
    let initial_state = state.lock().unwrap().clone();
    let provider = CssProvider::new();
    provider.load_from_data(CSS);
    gtk::style_context_add_provider_for_display(
        &gtk::gdk::Display::default().expect("Display error"),
        &provider,
        gtk::STYLE_PROVIDER_PRIORITY_APPLICATION,
    );
    let window = ApplicationWindow::builder()
        .application(app)
        .title("ToTray")
        .default_width(320)
        .default_height(600)
        .resizable(true)
        .build();
    window.set_icon_name(Some(crate::state::APP_ID));

    let root_vbox = GBox::new(Orientation::Vertical, 0);
    root_vbox.add_css_class("main-box");
    root_vbox.set_halign(Align::Fill);
    root_vbox.set_hexpand(true);
    window.set_child(Some(&root_vbox));

    let header_box = GBox::new(Orientation::Horizontal, 8);
    header_box.add_css_class("header-box");

    let pb_loader = PixbufLoader::new();
    pb_loader.write(include_bytes!("../assets/logo.png")).ok();
    pb_loader.close().ok();
    let icon = Image::builder().pixel_size(44).build();
    if let Some(pb) = pb_loader.pixbuf() {
        icon.set_from_pixbuf(Some(&pb));
    }
    header_box.append(&icon);

    let title_vbox = GBox::new(Orientation::Vertical, 0);
    let title_hbox = GBox::new(Orientation::Horizontal, 0);
    title_hbox.set_valign(Align::Center);
    title_hbox.append(
        &Label::builder()
            .label("ToTray")
            .ellipsize(gtk::pango::EllipsizeMode::End)
            .css_classes(["app-title"])
            .build(),
    );

    let combined_btn = Button::builder()
        .css_classes(["version-btn"])
        .valign(Align::Center)
        .has_frame(false)
        .build();

    let btn_content = GBox::new(Orientation::Horizontal, 4);
    btn_content.append(
        &Label::builder()
            .label(format!("v{CURRENT_VERSION}"))
            .ellipsize(gtk::pango::EllipsizeMode::End)
            .build(),
    );
    let settings_icon = Image::from_icon_name("preferences-system-symbolic");
    settings_icon.set_pixel_size(15);
    btn_content.append(&settings_icon);
    combined_btn.set_child(Some(&btn_content));
    title_hbox.append(&combined_btn);

    title_vbox.append(&title_hbox);
    title_vbox.append(&Label::builder()
        .use_markup(true)
        .label("<b>AutoRun utility for hyprland</b> • by <a href='https://github.com/agzes'>agzes</a>")
        .halign(Align::Start)
        .wrap(true)
        .css_classes(["app-subtitle"])
        .build());
    header_box.append(&title_vbox);

    let filler = GBox::new(Orientation::Horizontal, 0);
    filler.set_hexpand(true);
    header_box.append(&filler);

    let right_vbox = GBox::new(Orientation::Vertical, 4);
    right_vbox.set_valign(Align::Center);
    right_vbox.set_halign(Align::End);

    let status_badge = Label::builder()
        .label(if initial_state.auto_start {
            "ACTIVE"
        } else {
            "IDLE"
        })
        .css_classes([
            "status-badge",
            if initial_state.auto_start {
                "active"
            } else {
                "inactive"
            },
        ])
        .halign(Align::End)
        .build();
    right_vbox.append(&status_badge);

    right_vbox.append(
        &Label::builder()
            .use_markup(true)
            .label("<a href='https://github.com/agzes/ToTray'>GITHUB</a>")
            .css_classes(["badge-link"])
            .halign(Align::End)
            .build(),
    );
    header_box.append(&right_vbox);
    root_vbox.append(&header_box);

    let stack = Stack::builder()
        .transition_type(gtk::StackTransitionType::Crossfade)
        .transition_duration(400)
        .vexpand(true)
        .hexpand(true)
        .build();

    let main_vbox = GBox::new(Orientation::Vertical, 0);
    main_vbox.set_hexpand(true);

    let compat_scrolled = gtk::ScrolledWindow::builder()
        .hscrollbar_policy(gtk::PolicyType::Never)
        .vexpand(true)
        .min_content_height(100)
        .build();
    let compat_vbox = GBox::new(Orientation::Vertical, 0);
    compat_vbox.set_hexpand(true);
    compat_scrolled.set_child(Some(&compat_vbox));

    let warning_scrolled = gtk::ScrolledWindow::builder()
        .hscrollbar_policy(gtk::PolicyType::Never)
        .vexpand(true)
        .min_content_height(100)
        .build();
    let warning_vbox = GBox::new(Orientation::Vertical, 0);
    warning_vbox.set_hexpand(true);
    warning_scrolled.set_child(Some(&warning_vbox));

    let desktop_scrolled = gtk::ScrolledWindow::builder()
        .hscrollbar_policy(gtk::PolicyType::Never)
        .vexpand(true)
        .min_content_height(100)
        .build();
    let desktop_vbox = GBox::new(Orientation::Vertical, 0);
    desktop_vbox.set_hexpand(true);
    desktop_scrolled.set_child(Some(&desktop_vbox));

    let warning_spacer_top = GBox::new(Orientation::Vertical, 0);
    warning_spacer_top.set_vexpand(true);
    warning_vbox.append(&warning_spacer_top);

    let warning_content = GBox::new(Orientation::Vertical, 0);
    warning_content.set_halign(Align::Center);

    let welcome_icon = Image::from_icon_name("dialog-warning-symbolic");
    welcome_icon.set_pixel_size(80);
    welcome_icon.set_margin_bottom(20);
    welcome_icon.set_halign(Align::Center);
    warning_content.append(&welcome_icon);

    warning_content.append(
        &Label::builder()
            .label("ToTray")
            .ellipsize(gtk::pango::EllipsizeMode::End)
            .css_classes(["welcome-title"])
            .halign(Align::Center)
            .build(),
    );
    warning_content.append(
        &Label::builder()
            .label("Auto-start & Tray Manager")
            .css_classes(["welcome-subtitle"])
            .halign(Align::Center)
            .wrap(true)
            .build(),
    );

    let note_box = GBox::new(Orientation::Vertical, 8);
    note_box.add_css_class("info-note");
    note_box.set_halign(Align::Center);
    note_box.append(&Label::builder()
        .use_markup(true)
        .label("<span size='large' weight='800'>Hyprland Only</span>\n\nThis tool is designed specifically for Hyprland using hyprctl dispatch commands.")
        .justify(gtk::Justification::Center)
        .wrap(true)
        .build());
    warning_content.append(&note_box);

    warning_vbox.append(&warning_content);

    let warning_spacer_bottom = GBox::new(Orientation::Vertical, 0);
    warning_spacer_bottom.set_vexpand(true);
    warning_vbox.append(&warning_spacer_bottom);

    let ok_btn = Button::builder()
        .label("Get Started")
        .css_classes(["start-button"])
        .build();
    let s_clone = stack.clone();
    let st_clone = state.clone();
    ok_btn.connect_clicked(move |_| {
        let mut s = st_clone.lock().unwrap();
        s.shown_warning = true;
        s.save();

        if !s.desktop_installed || !crate::backend::desktop_file_exists() {
            s_clone.set_visible_child_name("desktop");
        } else {
            s_clone.set_visible_child_name("main");
        }
    });
    warning_vbox.append(&ok_btn);

    let desktop_spacer_top = GBox::new(Orientation::Vertical, 0);
    desktop_spacer_top.set_vexpand(true);
    desktop_vbox.append(&desktop_spacer_top);

    let desktop_content = GBox::new(Orientation::Vertical, 0);
    desktop_content.set_halign(Align::Center);

    let d_icon = Image::from_icon_name("system-run-symbolic");
    d_icon.set_pixel_size(80);
    d_icon.set_margin_bottom(20);
    d_icon.set_halign(Align::Center);
    desktop_content.append(&d_icon);

    desktop_content.append(
        &Label::builder()
            .label("Desktop Integration")
            .css_classes(["welcome-title"])
            .halign(Align::Center)
            .build(),
    );
    desktop_content.append(
        &Label::builder()
            .label("For proper Auto-Start and Tray functionality")
            .css_classes(["welcome-subtitle"])
            .halign(Align::Center)
            .wrap(true)
            .build(),
    );

    let d_note_box = GBox::new(Orientation::Vertical, 8);
    d_note_box.add_css_class("info-note");
    d_note_box.set_halign(Align::Center);
    d_note_box.append(&Label::builder()
        .use_markup(true)
        .label("<span size='large' weight='800'>Desktop Shortcut</span>\n\nToTray needs to be registered as an application to support system tray protocols and reliable auto-start.")
        .justify(gtk::Justification::Center)
        .wrap(true)
        .build());

    if !crate::backend::is_in_path() {
        let fix_path_btn = Button::builder()
            .label("Fix Terminal Command")
            .css_classes(["version-btn"])
            .halign(Align::Center)
            .build();
        fix_path_btn.connect_clicked(move |btn| {
            if crate::backend::add_to_path_config() {
                btn.set_label("PATH Fixed (Restart Terminal)");
                btn.set_sensitive(false);
            }
        });
        d_note_box.append(&fix_path_btn);
        d_note_box.append(&Label::builder()
            .use_markup(true)
            .label("<span color='#f5c71a' weight='bold'>Note:</span> ~/.local/bin is not in your PATH. Click above to fix or add it manually to your .bashrc.")
            .justify(gtk::Justification::Center)
            .wrap(true)
            .build());
    }

    desktop_content.append(&d_note_box);
    desktop_vbox.append(&desktop_content);

    let desktop_spacer_bottom = GBox::new(Orientation::Vertical, 0);
    desktop_spacer_bottom.set_vexpand(true);
    desktop_vbox.append(&desktop_spacer_bottom);

    let install_d_btn = Button::builder()
        .label("Install Desktop File")
        .css_classes(["start-button"])
        .build();
    let skip_d_btn = Button::builder()
        .label("Skip")
        .css_classes(["version-btn"])
        .halign(Align::Center)
        .build();

    let s_c_d = stack.clone();
    let st_c_d = state.clone();
    install_d_btn.connect_clicked(move |_| {
        if crate::backend::setup_desktop_file() {
            let mut s = st_c_d.lock().unwrap();
            s.desktop_installed = true;
            s.save();
            s_c_d.set_visible_child_name("main");
        }
    });

    let s_c_d2 = stack.clone();
    let st_c_d2 = state.clone();
    skip_d_btn.connect_clicked(move |_| {
        let mut s = st_c_d2.lock().unwrap();
        s.desktop_installed = true;
        s.save();
        s_c_d2.set_visible_child_name("main");
    });

    desktop_vbox.append(&skip_d_btn);
    desktop_vbox.append(&install_d_btn);

    let main_scrolled = gtk::ScrolledWindow::builder()
        .vexpand(true)
        .hscrollbar_policy(gtk::PolicyType::Never)
        .min_content_height(100)
        .build();
    let main_scroll_content = GBox::new(Orientation::Vertical, 0);
    main_scrolled.set_child(Some(&main_scroll_content));
    main_vbox.append(&main_scrolled);

    main_scroll_content.append(
        &Label::builder()
            .label("Active Rules")
            .css_classes(["section-title"])
            .margin_start(4)
            .build(),
    );
    let rules_list = ListBox::new();
    rules_list.add_css_class("card");
    rules_list.set_hexpand(true);
    rules_list.set_selection_mode(gtk::SelectionMode::None);
    rules_list.set_can_focus(false);

    let empty_row = ListBoxRow::builder()
        .activatable(false)
        .selectable(false)
        .can_focus(false)
        .build();
    let empty_label = Label::builder()
        .label("No rules defined yet.")
        .css_classes(["row-subtitle"])
        .margin_top(20)
        .margin_bottom(20)
        .halign(Align::Center)
        .build();
    empty_row.set_child(Some(&empty_label));
    empty_row.set_visible(initial_state.apps.is_empty());

    rules_list.append(&empty_row);

    for rule in &initial_state.apps {
        rules_list.append(&create_rule_row(
            rule,
            state.clone(),
            &rules_list,
            &empty_row,
        ));
    }

    main_scroll_content.append(&rules_list);

    let add_card = GBox::new(Orientation::Vertical, 0);
    add_card.add_css_class("bottom-card");
    add_card.set_margin_top(10);
    main_vbox.append(&add_card);

    let expander_header = GBox::new(Orientation::Horizontal, 8);
    expander_header.add_css_class("expander-header");

    let header_icon = Image::from_icon_name("list-add-symbolic");
    header_icon.set_pixel_size(18);
    expander_header.append(&header_icon);

    let header_title = Label::builder()
        .label("Add New Rule")
        .css_classes(["expander-title"])
        .hexpand(true)
        .halign(Align::Start)
        .build();
    expander_header.append(&header_title);

    let settings_btn = Button::builder()
        .icon_name("preferences-system-symbolic")
        .css_classes(["icon-btn"])
        .valign(Align::Center)
        .has_frame(false)
        .tooltip_text("Settings & Compatibility")
        .build();
    expander_header.append(&settings_btn);

    let arrow_icon = Image::from_icon_name("pan-down-symbolic");
    arrow_icon.add_css_class("expander-icon");
    expander_header.append(&arrow_icon);

    add_card.append(&expander_header);

    let revealer = Revealer::builder()
        .transition_type(gtk::RevealerTransitionType::SlideDown)
        .transition_duration(350)
        .build();
    add_card.append(&revealer);

    let add_vbox = GBox::new(Orientation::Vertical, 6);
    add_vbox.add_css_class("reveal-content");
    revealer.set_child(Some(&add_vbox));

    let gesture = GestureClick::new();
    let rev_c = revealer.clone();
    let arrow_c = arrow_icon.clone();
    gesture.connect_released(move |_, _, _, _| {
        let is_expanded = !rev_c.reveals_child();
        rev_c.set_reveal_child(is_expanded);
        if is_expanded {
            arrow_c.add_css_class("expanded");
        } else {
            arrow_c.remove_css_class("expanded");
        }
    });
    expander_header.add_controller(gesture);

    let add_list = ListBox::new();
    add_list.add_css_class("card");
    add_list.set_hexpand(true);
    let name_in = Entry::builder().placeholder_text("firefox").build();
    let exec_in = Entry::builder().placeholder_text("firefox").build();

    let action_list_factory = SignalListItemFactory::new();
    action_list_factory.connect_setup(move |_, list_item| {
        let box_ = GBox::new(Orientation::Vertical, 0);
        let title = Label::builder()
            .halign(Align::Start)
            .use_markup(true)
            .build();
        let subtitle = Label::builder()
            .halign(Align::Start)
            .css_classes(["row-subtitle"])
            .build();
        box_.append(&title);
        box_.append(&subtitle);
        list_item
            .downcast_ref::<ListItem>()
            .unwrap()
            .set_child(Some(&box_));
    });
    action_list_factory.connect_bind(move |_, list_item| {
        let item = list_item.downcast_ref::<ListItem>().unwrap();
        let box_ = item.child().unwrap().downcast::<GBox>().unwrap();
        let title = box_.first_child().unwrap().downcast::<Label>().unwrap();
        let subtitle = title.next_sibling().unwrap().downcast::<Label>().unwrap();
        match item.position() {
            0 => {
                title.set_markup("<b>To Tray</b>");
                subtitle.set_label("Minimize matching windows to the system tray");
            }
            1 => {
                title.set_markup("<b>Close</b>");
                subtitle.set_label("Close matching windows as soon as they appear");
            }
            2 => {
                title.set_markup("<b>Close 2x</b>");
                subtitle.set_label("Close two windows (useful for Discord/Vesktop splash)");
            }
            3 => {
                title.set_markup("<b>Workspace</b>");
                subtitle.set_label("Automatically move window to a specific workspace");
            }
            _ => {}
        }
    });

    let action_selected_factory = SignalListItemFactory::new();
    action_selected_factory.connect_setup(move |_, list_item| {
        let label = Label::builder().halign(Align::Start).build();
        list_item
            .downcast_ref::<ListItem>()
            .unwrap()
            .set_child(Some(&label));
    });
    action_selected_factory.connect_bind(move |_, list_item| {
        let item = list_item.downcast_ref::<ListItem>().unwrap();
        let label = item.child().unwrap().downcast::<Label>().unwrap();
        label.set_label(match item.position() {
            0 => "To Tray",
            1 => "Close",
            2 => "Close 2x",
            3 => "Workspace",
            _ => "",
        });
    });

    let act_drop = DropDown::builder()
        .model(&StringList::new(&[
            "To Tray",
            "Close",
            "Close 2x",
            "Workspace",
        ]))
        .factory(&action_selected_factory)
        .list_factory(&action_list_factory)
        .build();

    let ws_in = Entry::builder()
        .placeholder_text("1")
        .width_request(40)
        .sensitive(false)
        .build();
    let ws_c = ws_in.clone();
    act_drop.connect_selected_notify(move |d| ws_c.set_sensitive(d.selected() == 3));

    add_list.append(&create_row(
        "Window Class",
        Some("The unique ID of the application window"),
        &name_in,
        Some("Use 'hyprctl clients' to find the 'class' of your window"),
    ));
    add_list.append(&create_row(
        "Exec Command",
        Some("The command used to start the app"),
        &exec_in,
        Some("Example: 'firefox' or '/usr/bin/flatpak run...'"),
    ));
    add_list.append(&create_row(
        "Action",
        Some("What happens when the window is found"),
        &act_drop,
        None,
    ));
    add_list.append(&create_row(
        "Workspace #",
        Some("Target workspace number"),
        &ws_in,
        Some("Only applies if the 'Workspace' action is selected"),
    ));
    add_list.set_margin_bottom(24);
    add_vbox.append(&add_list);

    let add_btn = Button::builder()
        .label("Add Rule")
        .css_classes(["start-button"])
        .build();
    add_vbox.append(&add_btn);

    let st_c = state.clone();
    let list_c = rules_list.clone();
    let er_c = empty_row.clone();
    add_btn.connect_clicked(move |_| {
        let name = name_in.text().to_string();
        let exec = exec_in.text().to_string();
        if name.is_empty() || exec.is_empty() {
            return;
        }

        let action = match act_drop.selected() {
            0 => Action::HideToTray,
            1 => Action::Close,
            2 => Action::Close2,
            _ => Action::Workspace(ws_in.text().parse().unwrap_or(1)),
        };

        let rule = AppRule { name, exec, action };
        let mut s = st_c.lock().unwrap();
        s.apps.push(rule.clone());
        s.save();
        list_c.append(&create_rule_row(&rule, st_c.clone(), &list_c, &er_c));
        er_c.set_visible(false);

        name_in.set_text("");
        exec_in.set_text("");
        ws_in.set_text("");
    });

    let compat_vbox_clone = compat_vbox.clone();
    let stack_clone = stack.clone();
    let state_clone = state.clone();
    let badge_clone = status_badge.clone();
    let window_c = window.clone();
    let refresh_compat = move || {
        while let Some(child) = compat_vbox_clone.first_child() {
            compat_vbox_clone.remove(&child);
        }
        build_compat_ui(
            window_c.clone(),
            compat_vbox_clone.clone(),
            stack_clone.clone(),
            state_clone.clone(),
            badge_clone.clone(),
        );
        stack_clone.set_visible_child_name("compat");
    };

    stack.add_named(&main_vbox, Some("main"));
    stack.add_named(&compat_scrolled, Some("compat"));
    stack.add_named(&warning_scrolled, Some("warning"));
    stack.add_named(&desktop_scrolled, Some("desktop"));
    root_vbox.append(&stack);

    let rc = refresh_compat.clone();
    combined_btn.connect_clicked(move |_| {
        rc();
    });

    let rc2 = refresh_compat.clone();
    settings_btn.connect_clicked(move |_| {
        rc2();
    });

    let (shown_warning, desktop_installed, last_v) = {
        let s = state.lock().unwrap();
        (
            s.shown_warning,
            s.desktop_installed,
            s.last_run_version.clone(),
        )
    };

    let desktop_exists = crate::backend::desktop_file_exists();

    if !shown_warning {
        stack.set_visible_child_name("warning");
    } else if !desktop_installed || !desktop_exists {
        stack.set_visible_child_name("desktop");
    } else if last_v.is_none_or(|v| v != CURRENT_VERSION) {
        refresh_compat();
    } else {
        stack.set_visible_child_name("main");
    }

    window.connect_close_request(move |win| {
        win.set_visible(false);
        glib::Propagation::Stop
    });

    window
}

fn check_latest_version() -> Result<(bool, String), String> {
    let remote_v = Command::new("curl")
        .args([
            "-s",
            "--fail",
            "--connect-timeout",
            "3",
            "https://raw.githubusercontent.com/agzes/ToTray/main/version",
        ])
        .output();

    if let Ok(output) = remote_v
        && output.status.success()
    {
        let latest_v_str = String::from_utf8_lossy(&output.stdout).trim().to_string();
        if !latest_v_str.is_empty() && !latest_v_str.contains("Not Found") {
            return Ok((CURRENT_VERSION == latest_v_str, latest_v_str));
        }
    }

    Err("Failed to check for updates".to_string())
}

fn add_compat_item(
    name: &str,
    tutorial: &str,
    widget: Option<gtk::Widget>,
    status: ItemStatus,
) -> GBox {
    let item = GBox::new(Orientation::Vertical, 2);
    item.add_css_class("compat-item");
    let icon_name = match status {
        ItemStatus::Ok => {
            item.add_css_class("ok");
            "emblem-ok-symbolic"
        }
        ItemStatus::Error => {
            item.add_css_class("error");
            "dialog-error-symbolic"
        }
        ItemStatus::Warning => {
            item.add_css_class("warning-item");
            "dialog-warning-symbolic"
        }
        ItemStatus::Info => {
            item.add_css_class("info-item");
            "dialog-information-symbolic"
        }
    };

    let header = GBox::new(Orientation::Horizontal, 10);
    header.append(&Image::from_icon_name(icon_name));
    header.append(
        &Label::builder()
            .label(name)
            .css_classes(["compat-name"])
            .wrap(true)
            .xalign(0.0)
            .ellipsize(gtk::pango::EllipsizeMode::End)
            .build(),
    );

    let filler = GBox::new(Orientation::Horizontal, 0);
    filler.set_hexpand(true);
    header.append(&filler);

    if let Some(w) = widget {
        header.append(&w);
    }

    item.append(&header);
    let tut = Label::builder()
        .label(tutorial)
        .css_classes(["tutorial-text"])
        .halign(Align::Start)
        .wrap(true)
        .xalign(0.0)
        .build();
    item.append(&tut);
    item
}

#[derive(Clone, Copy)]
enum ItemStatus {
    Ok,
    Error,
    Warning,
    Info,
}
fn build_compat_ui(
    window: ApplicationWindow,
    container: GBox,
    stack: Stack,
    state: SharedState,
    status_badge: Label,
) {
    let status_badge_v = status_badge.clone();

    container.append(
        &Label::builder()
            .label("Compatibility")
            .css_classes(["compat-title"])
            .halign(Align::Start)
            .wrap(true)
            .xalign(0.0)
            .build(),
    );
    let list = GBox::new(Orientation::Vertical, 0);
    container.append(&list);

    let version_box = GBox::new(Orientation::Vertical, 0);
    list.append(&version_box);
    version_box.append(&add_compat_item(
        "Application Version",
        "Checking for updates...",
        None,
        ItemStatus::Ok,
    ));

    #[allow(deprecated)]
    let (tx, rx) =
        glib::MainContext::channel::<Result<(bool, String), String>>(glib::Priority::DEFAULT);

    let tx_thread = tx.clone();
    std::thread::spawn(move || {
        let res = check_latest_version();
        let _ = tx_thread.send(res);
    });

    let version_box_v = version_box.clone();
    let stack_v = stack.clone();
    let state_v = state.clone();
    let container_v = container.clone();
    let window_v = window.clone();

    rx.attach(None, move |result| {
        while let Some(child) = version_box_v.first_child() {
            version_box_v.remove(&child);
        }

        let render_version = |res: Result<(bool, String), String>,
                              vb: GBox,
                              s: Stack,
                              st: SharedState,
                              c: GBox,
                              sb: Label,
                              w: ApplicationWindow| {
            while let Some(child) = vb.first_child() {
                vb.remove(&child);
            }
            match res {
                Ok((version_ok, latest_v)) => {
                    let version_tutorial = if version_ok {
                        format!("Current: v{CURRENT_VERSION}. You have the latest version.")
                    } else {
                        format!("Update available: v{latest_v}. Visit GitHub to download.")
                    };
                    let check_btn = Button::builder()
                        .label(if version_ok { "CHECK" } else { "UPDATE" })
                        .css_classes(["version-btn"])
                        .valign(Align::Center)
                        .build();
                    let s_c = s.clone();
                    let st_c = st.clone();
                    let c_c = c.clone();
                    let sb_c = sb.clone();
                    let w_c = w.clone();
                    let v_ok = version_ok;
                    check_btn.connect_clicked(move |_| {
                        if v_ok {
                            while let Some(child) = c_c.first_child() {
                                c_c.remove(&child);
                            }
                            build_compat_ui(
                                w_c.clone(),
                                c_c.clone(),
                                s_c.clone(),
                                st_c.clone(),
                                sb_c.clone(),
                            );
                        } else {
                            let _ = Command::new("xdg-open")
                                .arg("https://github.com/agzes/ToTray/releases")
                                .spawn();
                        }
                    });
                    let status = if version_ok {
                        ItemStatus::Ok
                    } else {
                        ItemStatus::Info
                    };
                    vb.append(&add_compat_item(
                        "Application Version",
                        &version_tutorial,
                        Some(check_btn.upcast()),
                        status,
                    ));
                }
                Err(e) => {
                    let retry_btn = Button::builder()
                        .label("RETRY")
                        .css_classes(["version-btn"])
                        .valign(Align::Center)
                        .build();
                    let s_c = s.clone();
                    let st_c = st.clone();
                    let c_c = c.clone();
                    let sb_c = sb.clone();
                    let w_c = w.clone();
                    retry_btn.connect_clicked(move |_| {
                        while let Some(child) = c_c.first_child() {
                            c_c.remove(&child);
                        }
                        build_compat_ui(
                            w_c.clone(),
                            c_c.clone(),
                            s_c.clone(),
                            st_c.clone(),
                            sb_c.clone(),
                        );
                    });
                    vb.append(&add_compat_item(
                        "Application Version",
                        &e,
                        Some(retry_btn.upcast()),
                        ItemStatus::Warning,
                    ));
                }
            }
        };

        render_version(
            result,
            version_box_v.clone(),
            stack_v.clone(),
            state_v.clone(),
            container_v.clone(),
            status_badge_v.clone(),
            window_v.clone(),
        );

        glib::ControlFlow::Break
    });

    let is_hyprland = crate::backend::is_hyprland();
    let status = if is_hyprland {
        ItemStatus::Ok
    } else {
        ItemStatus::Error
    };
    list.append(&add_compat_item(
        "Hyprland Environment",
        "ToTray requires Hyprland to manage windows.",
        None,
        status,
    ));

    let desktop_ok = crate::backend::desktop_file_exists();
    let d_status = if desktop_ok {
        ItemStatus::Ok
    } else {
        ItemStatus::Warning
    };
    let d_fix = if !desktop_ok {
        let fix_btn = Button::builder()
            .label("FIX")
            .css_classes(["version-btn"])
            .valign(Align::Center)
            .build();
        let s_c = stack.clone();
        let st_c = state.clone();
        let sb_c = status_badge.clone();
        let c_c = container.clone();
        let w_c = window.clone();
        fix_btn.connect_clicked(move |_| {
            if crate::backend::setup_desktop_file() {
                {
                    let mut s = st_c.lock().unwrap();
                    s.desktop_installed = true;
                    s.save();
                }
                while let Some(child) = c_c.first_child() {
                    c_c.remove(&child);
                }
                build_compat_ui(
                    w_c.clone(),
                    c_c.clone(),
                    s_c.clone(),
                    st_c.clone(),
                    sb_c.clone(),
                );
            }
        });
        Some(fix_btn.upcast::<gtk::Widget>())
    } else {
        None
    };

    list.append(&add_compat_item(
        "Desktop Integration",
        "Registered as a system application for tray and auto-start support.",
        d_fix,
        d_status,
    ));

    container.append(
        &Label::builder()
            .label("Settings")
            .css_classes(["compat-title"])
            .halign(Align::Start)
            .margin_top(24)
            .wrap(true)
            .xalign(0.0)
            .build(),
    );

    let initial_state = state.lock().unwrap().clone();
    let settings_list = ListBox::new();
    settings_list.add_css_class("card");
    settings_list.set_hexpand(true);
    settings_list.set_selection_mode(gtk::SelectionMode::None);
    settings_list.set_can_focus(false);
    container.append(&settings_list);

    let auto_sw = Switch::new();
    auto_sw.set_active(initial_state.auto_start);
    let st_c_auto = state.clone();
    let badge_c_auto = status_badge.clone();
    let stack_c_auto = stack.clone();
    auto_sw.connect_state_set(move |sw, state_val| {
        let is_installed = { st_c_auto.lock().unwrap().desktop_installed }
            || crate::backend::desktop_file_exists();

        if state_val && !is_installed {
            sw.set_active(false);
            stack_c_auto.set_visible_child_name("desktop");
            return glib::Propagation::Stop;
        }

        if state_val {
            badge_c_auto.set_label("ACTIVE");
            badge_c_auto.add_css_class("active");
            badge_c_auto.remove_css_class("inactive");
        } else {
            badge_c_auto.set_label("IDLE");
            badge_c_auto.add_css_class("inactive");
            badge_c_auto.remove_css_class("active");
        }
        let mut s = st_c_auto.lock().unwrap();
        s.auto_start = state_val;
        s.save();
        crate::backend::autostart(state_val);
        glib::Propagation::Proceed
    });
    settings_list.append(&create_row(
        "Auto-Start",
        Some("Launch with Hyprland"),
        &auto_sw,
        None,
    ));

    let notify_sw = Switch::new();
    notify_sw.set_active(initial_state.notifications);
    let st_c_notif = state.clone();
    notify_sw.connect_state_set(move |_, state_val| {
        let mut s = st_c_notif.lock().unwrap();
        s.notifications = state_val;
        s.save();
        glib::Propagation::Proceed
    });
    settings_list.append(&create_row(
        "Notifications",
        Some("Show app launch alerts"),
        &notify_sw,
        None,
    ));

    let silent_sw = Switch::new();
    silent_sw.set_active(initial_state.silent_mode);
    let st_c_silent = state.clone();
    silent_sw.connect_state_set(move |_, state_val| {
        let mut s = st_c_silent.lock().unwrap();
        s.silent_mode = state_val;
        s.save();
        glib::Propagation::Proceed
    });
    settings_list.append(&create_row(
        "Silent Mode",
        Some("Start hidden in background"),
        &silent_sw,
        None,
    ));

    let adj = Adjustment::new(
        initial_state.launch_delay as f64,
        0.0,
        5000.0,
        100.0,
        500.0,
        0.0,
    );
    let delay_spin = SpinButton::new(Some(&adj), 1.0, 0);
    let st_c_delay = state.clone();
    delay_spin.connect_value_changed(move |s_btn| {
        let mut s = st_c_delay.lock().unwrap();
        s.launch_delay = s_btn.value() as u64;
        s.save();
    });
    settings_list.append(&create_row(
        "Launch Delay (ms)",
        Some("Wait before action"),
        &delay_spin,
        None,
    ));

    container.append(
        &Label::builder()
            .label("Configuration")
            .css_classes(["compat-title"])
            .halign(Align::Start)
            .margin_top(24)
            .wrap(true)
            .xalign(0.0)
            .build(),
    );
    let config_list = ListBox::new();
    config_list.add_css_class("card");
    config_list.set_hexpand(true);
    config_list.set_selection_mode(gtk::SelectionMode::None);
    config_list.set_can_focus(false);
    container.append(&config_list);

    let export_btn = Button::builder()
        .label("Export")
        .css_classes(["version-btn"])
        .valign(Align::Center)
        .build();
    let st_exp = state.clone();
    let win_exp = window.clone();
    export_btn.connect_clicked(move |_| {
        let dialog = FileDialog::builder()
            .title("Export Configuration")
            .initial_name("totray_config.json")
            .build();

        let st = st_exp.clone();
        dialog.save(Some(&win_exp), gtk::gio::Cancellable::NONE, move |res| {
            if let Ok(file) = res {
                let path = file.path().unwrap();
                let s = st.lock().unwrap();
                let _ = s.export_config(path);
            }
        });
    });
    config_list.append(&create_row(
        "Export Rules",
        Some("Save your rules to a file"),
        &export_btn,
        None,
    ));

    let import_btn = Button::builder()
        .label("Import")
        .css_classes(["version-btn"])
        .valign(Align::Center)
        .build();
    let st_imp = state.clone();
    let win_imp = window.clone();
    let stack_imp = stack.clone();
    let container_imp = container.clone();
    let badge_imp = status_badge.clone();

    import_btn.connect_clicked(move |_| {
        let filter = FileFilter::new();
        filter.add_pattern("*.json");
        filter.set_name(Some("JSON files"));

        let filters = gtk::gio::ListStore::new::<FileFilter>();
        filters.append(&filter);

        let dialog = FileDialog::builder()
            .title("Import Configuration")
            .filters(&filters)
            .build();

        let st = st_imp.clone();
        let w = win_imp.clone();
        let s = stack_imp.clone();
        let c = container_imp.clone();
        let b = badge_imp.clone();

        dialog.open(Some(&win_imp), gtk::gio::Cancellable::NONE, move |res| {
            if let Ok(file) = res {
                let path = file.path().unwrap();
                let mut s_lock = st.lock().unwrap();
                if s_lock.import_config(path).is_ok() {
                    drop(s_lock);
                    while let Some(child) = c.first_child() {
                        c.remove(&child);
                    }
                    build_compat_ui(w.clone(), c, s, st, b);
                }
            }
        });
    });
    config_list.append(&create_row(
        "Import Rules",
        Some("Load rules from a file"),
        &import_btn,
        None,
    ));

    let uninstall_btn = Button::builder()
        .label("Uninstall (Remove all traces)")
        .css_classes(["stop-button"])
        .margin_top(20)
        .build();
    let st_un = state.clone();
    let badge_un = status_badge.clone();
    let s_un = stack.clone();
    let c_un = container.clone();
    let w_un = window.clone();
    uninstall_btn.connect_clicked(move |btn| {
        if crate::backend::uninstall() {
            {
                let mut s = st_un.lock().unwrap();
                s.auto_start = false;
                s.desktop_installed = false;
                s.save();
            }
            badge_un.set_label("IDLE");
            badge_un.add_css_class("inactive");
            badge_un.remove_css_class("active");
            btn.set_label("Uninstalled successfully!");
            btn.set_sensitive(false);

            let c_un_clone = c_un.clone();
            let s_un_clone = s_un.clone();
            let st_un_clone = st_un.clone();
            let badge_un_clone = badge_un.clone();
            let w_un_inner = w_un.clone();

            glib::timeout_add_local_once(std::time::Duration::from_millis(1500), move || {
                while let Some(child) = c_un_clone.first_child() {
                    c_un_clone.remove(&child);
                }
                build_compat_ui(
                    w_un_inner,
                    c_un_clone,
                    s_un_clone,
                    st_un_clone,
                    badge_un_clone,
                );
            });
        }
    });
    container.append(&uninstall_btn);

    let spacer = GBox::new(Orientation::Vertical, 0);
    spacer.set_vexpand(true);
    container.append(&spacer);

    let continue_btn = Button::builder()
        .label("Return to Dashboard")
        .css_classes(["start-button"])
        .margin_top(10)
        .build();
    let stack_clone = stack.clone();
    let state_clone = state.clone();
    continue_btn.connect_clicked(move |_| {
        let mut s = state_clone.lock().unwrap();
        s.last_run_version = Some(CURRENT_VERSION.to_string());
        s.save();
        stack_clone.set_visible_child_name("main");
    });
    container.append(&continue_btn);
}
