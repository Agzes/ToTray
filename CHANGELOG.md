# Changelog

## [Unreleased]

### Added

- **CI**: GitHub Actions workflow that builds `ToTray-vX.Y.Z.AppImage` and attaches it to the release when a `v*` tag is pushed.

### Changed

- **Auto-Start**: now managed as a systemd user service (`~/.config/systemd/user/totray.service`) instead of writing `exec-once` into `hyprland.conf`; legacy entries are cleaned up and migrated automatically.
- **Worker**: `--worker` runs headless (no GTK/display required) and waits until a live Hyprland instance reports ready monitors before running the rules.
- **Launching**: applications are started through Hyprland (`hyprctl dispatch exec`), so they inherit the full session environment; the launch delay is now applied to `Close2` as well, so windows are not closed while an app is still starting.
- **Service**: binary installs use an atomic replace, so uninstall/reinstall can no longer break the unit's ExecStart path.

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
