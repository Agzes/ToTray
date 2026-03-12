use std::process::Command;

pub fn close_window(class: &str) {
    Command::new("hyprctl")
        .args(["dispatch", "closewindow", &format!("class:{}", class)])
        .spawn()
        .ok();
}

pub fn move_workspace(class: &str, ws: u32) {
    Command::new("hyprctl")
        .args([
            "dispatch",
            "movetoworkspacesilent",
            &format!("{},class:{}", ws, class),
        ])
        .spawn()
        .ok();
}

pub fn hide_to_special(class: &str) {
    Command::new("hyprctl")
        .args([
            "dispatch",
            "movetoworkspacesilent",
            &format!("special,class:{}", class),
        ])
        .spawn()
        .ok();
}

pub fn show_from_special(class: &str) {
    Command::new("hyprctl")
        .args(["dispatch", "movetoworkspace", &format!("+0,class:{}", class)])
        .spawn()
        .ok();
}

pub fn get_window_count(class: &str) -> usize {
    let output = Command::new("hyprctl").args(["clients"]).output();

    if let Ok(out) = output {
        let s = String::from_utf8_lossy(&out.stdout);

        s.split('\n')
            .filter(|l| l.contains(&format!("class: {}", class)))
            .count()
    } else {
        0
    }
}
