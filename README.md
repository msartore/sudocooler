# sudocooler

`sudocooler` is a native app for Sudokoo CPU coolers, written in Rust with
[Native Shell](https://nsh.dev) (NSH 0.5). It reads live telemetry and drives
the ENERGLO 360 display. It runs on
Windows, Linux and macOS from one Rust crate, with no JavaScript VM.

The daemon itself is a **git submodule** at `sudokooler/`. This repository only
contains this app; the daemon keeps its own build, tests and hardware
safety rules.

## Layout

```text
src/
  domain/          std-only value objects: telemetry, devices, packets, settings
  application/     ports and use cases (no nsh, no processes)
  infrastructure/  CLI adapter for the daemon + deterministic demo adapter
  composition/     the only module that names concrete adapters
  ui/              styled NSH components (cards, pills, 7-segment display)
  presentation/    app shell and the four screens
sudokooler/        git submodule: the upstream Rust daemon
```

Dependencies point inward. `domain` cannot import NSH; `presentation` cannot
import process or platform code. The seam is `composition`.

## Screens

- **Overview** — live temperature, power, load and clock, plus a preview of the
  cooler's seven-segment panel and power bar. Pause/resume and rescan actions.
- **Devices** — detected `0x381C` HID interfaces with enabled/verified status.
  Unverified PIDs are shown but never writable.
- **Packets** — the 64-byte SK700V telemetry frame as contiguous hex, and a
  bounded log of recent frames.
- **Settings** — polling interval, demo/daemon switch, temperature unit and
  registry override.

## Run it

```sh
git submodule update --init --recursive

# Build/launch the daemon first if you want live hardware data:
cargo build --release --manifest-path sudokooler/Cargo.toml

# Run the UI (demo mode is the default; toggle it in Settings):
nsh run
# or, directly on this machine's desktop backend:
cargo run

# Checks
cargo test
nsh test
nsh check
```

The UI looks for the daemon binary in `SUDOKOOLER_BIN`, then in
`sudokooler/target/{release,debug}/`, then on `PATH`. If it is not found it
shows a readable error and stays in demo mode.

> Hardware safety is unchanged: only profiles with `enabled = true` and
> `VID == 0x381C` are ever written to. This UI does not enable unverified PIDs.

## Submodule

```sh
git submodule status
git -C sudokooler log -1 --oneline
```
