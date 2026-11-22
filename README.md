<div align="center">
  <img src="assets/logo.png" alt="sudocooler logo" width="120" />
  <h1>sudocooler</h1>
  <a href="https://github.com/msartore/sudocooler/actions/workflows/ci.yml">
    <img src="https://github.com/msartore/sudocooler/actions/workflows/ci.yml/badge.svg" alt="CI" />
  </a>
</div>

`sudocooler` is a native desktop app for Sudokoo CPU coolers. It shows live CPU
telemetry, streams frames to the cooler's display, and stays available from the
system tray. It is written in Rust with [Native Shell](https://nsh.dev) (NSH 0.5):
one crate for Windows, Linux and macOS, with no bundled JavaScript VM.

## Screenshot

![sudocooler overview](docs/screenshots/overview.png)

## Hardware

- **Sudokoo SK700V** (`0x381C:0x0003`) — verified seven-segment display.
- **Sudokoo ENERGLO 360** (`0x381C:0x0005`) — L120 display, supported by decoded
  telemetry frames (experimental).

The cooler displays numeric telemetry, not arbitrary images or wallpapers.

## Features

- **Live telemetry** — CPU temperature, package power, load and core clock with
  pause/resume and rescan.
- **L120 streaming** — a 19-byte telemetry frame is sent to the ENERGLO 360 on
  each tick; the display opens once and closes on quit.
- **One Home screen** — telemetry and a preview of the cooler's panel.
- **Home top bar** — **Settings** opens a modal with polling, temperature unit,
  autostart, self-update and about.
- **System tray** — closing the window hides the app into the tray; opening it
  again brings the existing instance forward.

## Build and run

```sh
git submodule update --init --recursive
cargo build --release --manifest-path sudokooler/Cargo.toml   # telemetry backend

nsh run            # or: cargo run
cargo check
cargo test
nsh test
nsh check
```

## Architecture

```
src/
  domain/          std-only value objects: telemetry, devices, packets, settings
  application/     ports and use cases (no nsh, no processes)
  infrastructure/  telemetry CLI, HID writer, tray/autostart, OS HID tree, updates
  composition/     the only module that names concrete adapters
  ui/              styled NSH components
  presentation/    home screen, settings modal and log window
sudokooler/        git submodule: the telemetry and hardware backend
docs/screenshots/  screenshots used in this README
```

Dependencies point inward: `domain` cannot import NSH, and `presentation` cannot
import process or platform code. `composition` is the seam.

### Telemetry source

Readings come from the bundled backend. On Windows, temperature and package power
need either an elevated PawnIO process or HWiNFO64 with shared memory; otherwise
they are reported as `0.0 °C / 0 W` rather than guessed. On Linux they come from
sysfs with no elevation; HID access needs the udev rule shipped in the submodule.

### Autostart

**Start at login** in Settings manages a Windows Scheduled Task named `sudocooler`
(`/SC ONLOGON /RL HIGHEST`, no console window), which provides the elevation the
Windows telemetry backend needs. On Linux the same setting writes an XDG entry at
`~/.config/autostart/sudocooler.desktop`.

### Updates

Packaged builds update from GitHub Releases. **Settings → Software update** reads
`latest.json`, and **Install update** downloads the verified archive, replaces the
app and restarts. NSH checks the app id, version, HTTPS URL and archive hash.
Releases publish `latest.json` via `nsh update-manifest`.

## Open source

- [Native Shell](https://nsh.dev) — Rust UI framework and CLI.
- [Tabler Icons](https://tabler.io/icons) — icons in `assets/icons/`.
- Rust crates including `hidapi`, `nsh-core`, `nsh-base`, the `nsh-*` platform
  backends, `env_logger`, `ureq`, `nsh-updater` and `winres` at build time.

## License

Licensed under the GNU General Public License version 3 (GPL-3.0). See
[LICENSE](LICENSE) for the full text.
