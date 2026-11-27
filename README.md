<div align="center">
  <kbd>🦀 Rust</kbd>
  <kbd>🧊 iced</kbd>
  <kbd>📂 Open-Source</kbd>
  <kbd>❄️ Hyprland</kbd>

<br><br><img src="assets/logo.png" width="128" alt="ToTray logo"/>
  <h1>ToTray <sup><kbd>v.1.0.0</kbd></sup></h1>
  <p><b>An automated application manager and tray utility for Hyprland.</b></p>

  <p>
    <a href="https://github.com/Agzes/totray/releases"><kbd>Latest release</kbd></a>
    &nbsp;·&nbsp;
    <a href="https://aur.archlinux.org/packages/totray"><kbd>AUR package</kbd></a>
    &nbsp;·&nbsp;
    <a href="#-build-from-source"><kbd>Build from source</kbd></a>
    &nbsp;·&nbsp;
    <a href="#-cli"><kbd>CLI</kbd></a>
  </p>
</div>

## > Features

- **Auto-Start Manager** - launches configured applications via a `systemd --user` service with optional startup delay and parallel or sequential execution.
- **Window Actions** - supports `To Tray`, `Close`, `Close X times` (configurable repeat count for splash screens like Discord/Vesktop/Steam), `Workspace`, `Special WS` (e.g. `special:shelf`), `Float`, `Fullscreen`, and `Launch Only`.
- **Tray Integration** - StatusNotifierItem tray (`ksni`) with quick restore for applications hidden to the `special` workspace.
- **Single-Instance IPC** - opening `totray` while the background daemon is active brings up the existing window instead of spawning duplicate tray icons.
- **Console Logs** - captures `stdout`/`stderr` output per rule directly inside the UI.
- **Native `iced` UI** - GPU-accelerated interface with dark/light theme support and optional GTK theme color synchronization.
- **CLI** - manage rules, desktop integration, and JSON output from the terminal (`totray --help`).

## > Installation

### AppImage

Download the latest AppImage from the [Releases page](https://github.com/Agzes/totray/releases), make it executable, and run it:

```bash
chmod +x ToTray-v1.0.0.AppImage
./ToTray-v1.0.0.AppImage
```

### Arch Linux (AUR)

```bash
yay -S totray
```

or

```bash
paru -S totray
```

## > Usage

### Desktop Integration & First Run

Launch the GUI:

```bash
./totray
```

On first launch, click **Add to System** (or run `./totray --install`) to install:

- Binary: `~/.local/bin/totray`
- Desktop entry: `~/.local/share/applications/totray.desktop`
- Icon: `~/.local/share/icons/hicolor/256x256/apps/totray.png`

Use `--tray` to start with the window hidden in the system tray.

### Background Mode (systemd service)

ToTray runs its background worker as a `systemd --user` service (`totray.service`) rather than `exec-once` in `hyprland.conf`.

- Toggle **Auto-Start** in **Settings** to install and enable `~/.config/systemd/user/totray.service`.
- Legacy `exec-once = ... totray` entries in `hyprland.conf` are automatically cleaned up and migrated.
- Manage the service manually if needed:

```bash
systemctl --user status totray
systemctl --user restart totray
journalctl --user -u totray -f
```

## > Build from source

Requires Rust, `pkg-config`, `libdbus`, `libxkbcommon`, and Vulkan/Wayland/X11 development headers.

- **Arch Linux**: `sudo pacman -S --needed base-devel pkg-config dbus libxkbcommon`
- **Debian / Ubuntu**: `sudo apt install build-essential pkg-config libdbus-1-dev libxkbcommon-dev`
- **Fedora**: `sudo dnf install gcc pkg-config dbus-devel libxkbcommon-devel`

```bash
git clone https://github.com/Agzes/totray.git
cd totray
cargo build --release
./target/release/totray
```

## > CLI

Add rules directly from your terminal:

```bash
# Hide Firefox to tray on launch
totray --add --name "firefox" --exec "firefox" --action "tray"

# Move Spotify to workspace 11
totray --add --name "spotify" --exec "spotify" --action "workspace" --workspace 11

# Close Vesktop 2 times (splash + main window)
totray --add --name "vesktop" --exec "vesktop" --action "closetimes" --count 2
```

### Flags

- `-w, --worker`: Start background worker (waits for Hyprland and runs rules).
- `-t, --tray`: Start minimized in the system tray.
- `--add`: Add a new rule via CLI (`--name`, `--exec`, `--action`, `--count`, `--workspace`, `--special`).
    - Actions: `tray`, `close`, `close2`, `closetimes`, `workspace`, `special`, `float`, `fullscreen`, `launch`.
- `--install`: Install desktop entry, icon, and binary to `~/.local/bin`.
- `--uninstall`: Remove desktop entry, icons, and binary.
- `--purge`: Remove all configuration files, systemd service, and desktop entries.
- `--config-json`: Print active rules in JSON format.
- `--version-json`: Print version info in JSON format.

## > Uninstall & Data Cleanup

- **Via UI**: Open **Settings -> Purge All Data & Traces -> Purge & Exit**.
- **Via CLI**:

```bash
totray --purge
```

For AUR installations, remove the package afterwards with `yay -Rns totray` or `paru -Rns totray`.

## > License

Distributed under the MIT License. See `LICENSE` for more information.

Icons by [Lucide](https://lucide.dev/) under the ISC License. See [assets/icons/LICENSE](assets/icons/LICENSE).

---

<kbd>With</kbd> <kbd>❤️</kbd> <kbd>by</kbd> <kbd>Agzes</kbd><br>
<kbd>pls ⭐ project!</kbd>
