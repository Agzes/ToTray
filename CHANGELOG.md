# Changelog

## [1.0.0] Iced Rewrite & Hyprland Actions - 2026-11-27

### Added

- **UI Rewrite (`iced` 0.14)**:
    - Replaced GTK4 with a native `iced` (`wgpu` / `tiny-skia`) interface.
    - Embedded Lucide vector SVG icons (`assets/icons/`, ISC + Feather MIT License).
    - Optional GTK theme & accent color synchronization and manual Dark/Light toggle.
    - In-app Diagnostics view (version check, Hyprland session, desktop integration, `PATH` check) and live Console view for rule logs.
    - Open window class picker (`Open...` from `hyprctl clients -j`) and in-place rule editing (`Edit Rule`).
- **New Window Actions & Repeat Support**:
    - `Close X times` (`closetimes --count N`) - sequentially closes `N` windows for apps with splash screens or updaters.
    - `Special WS` (`special --special <name>`) - moves matching windows to a named Hyprland special workspace (e.g. `special:shelf`).
    - `Float` (`float`), `Fullscreen` (`fullscreen`), and `Launch Only` (`launch`).
    - Configurable `Repeat` count (1-20, `--count N`) for `To Tray`, `Workspace`, and `Special WS`, tracking windows outside the target workspace so multi-window or reused-window apps (e.g. Steam, Vesktop) are handled reliably.
- **Single-Instance IPC & Desktop Lifecycle**:
    - Unix socket IPC (`$XDG_RUNTIME_DIR/totray.sock`) so launching `totray` focuses/opens the existing daemon window instead of spawning duplicate tray icons.
    - CLI flags `--tray`, `--install`, `--uninstall`, and `--purge`, plus an update prompt when `~/.local/bin/totray` differs from the running binary.
    - AUR package detection (`pacman -Qo`, `yay`/`paru`) with AUR update/uninstall banners, diagnostics, and modals.

### Changed

- **Hyprland Lua & Classic Support**: Added automatic `hyprctl eval` (`hl.dispatch(hl.dsp.*)`) fallback alongside classic `hyprctl dispatch`, plus `fullscreenstate 2 0` / `hl.dsp.window.fullscreen` for true fullscreen.
- **Window Matching & Restore**: `get_window_count` and `list_window_addresses` parse `hyprctl clients -j` with case-insensitive and substring fallbacks, and `show_from_special` queries `hyprctl activeworkspace -j`.
- **Systemd Worker & Launching**: `--worker` waits for Hyprland, `WAYLAND_DISPLAY`, and `DISPLAY` (syncing session variables from `systemctl --user show-environment`) before starting rules and UI; applications launch via `hyprctl` to inherit the full session environment.
- **Tray**: Upgraded `ksni` to `0.3.6` with `TrayBridge` state synchronization.

---

## [0.2.0] Systemd Release - 2026-09-22

### Added

- **CI**: GitHub Actions workflow that builds `ToTray-vX.Y.Z.AppImage` and attaches it to the release when a `v*` tag is pushed.

### Changed

- **Auto-Start**: now managed as a systemd user service (`~/.config/systemd/user/totray.service`) instead of writing `exec-once` into `hyprland.conf`; legacy entries are cleaned up and migrated automatically.
- **Worker**: `--worker` runs headless (no GTK/display required) and waits until a live Hyprland instance reports ready monitors before running the rules.
- **Launching**: applications are started through Hyprland (`hyprctl dispatch exec`), so they inherit the full session environment; the launch delay is now applied to `Close2` as well, so windows are not closed while an app is still starting.
- **Service**: binary installs use an atomic replace, so uninstall/reinstall can no longer break the unit's ExecStart path.

---

## [0.1.0] Init Release - 2026-03-14

### Added

- **GUI**: Modern GTK4 interface for managing rules and settings.
- **Tray**: Minimize applications to the system tray using `ksni`.
- **Hyprland Integration**: Automated window management and workspace rules.
- **Auto-Start Manager**: Configurable application startup with optional delays.
- **CLI**: Command-line interface for adding rules and querying status.
- **Notifications**: Desktop notifications for backend actions via `notify-rust`.
- **Multi-run Protection**: Prevent multiple instances of the worker or GUI.
- **Desktop Integration**: One-click "Install Desktop File" from the GUI.
