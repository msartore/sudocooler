//! Adapter that drives the `sudokooler` binary shipped as a git submodule.
//!
//! It uses the daemon's stable CLI instead of linking its internals: the
//! submodule stays a standalone project with its own safety guardrails, and
//! this UI keeps working if the daemon is built elsewhere.

use std::cell::RefCell;
use std::path::{Path, PathBuf};
use std::process::Command;

#[cfg(windows)]
use std::os::windows::process::CommandExt;
#[cfg(windows)]
const CREATE_NO_WINDOW: u32 = 0x0800_0000;

use crate::application::ports::{DeviceSource, TelemetrySource};
use crate::application::{AppError, ErrorKind};
use crate::domain::device::{Device, default_profiles};
use crate::domain::telemetry::Telemetry;

/// Runs the daemon's one-shot commands with an optional registry override.
pub struct CliDaemon {
    config_path: Option<String>,
    /// Sensor warning captured from the last dry-run's stderr (if any).
    notice: RefCell<Option<String>>,
}

impl CliDaemon {
    pub fn new(config_path: Option<String>) -> Self {
        Self {
            config_path,
            notice: RefCell::new(None),
        }
    }

    /// The sensor warning from the last `read()` call, if any.
    pub fn notice(&self) -> Option<String> {
        self.notice.borrow().clone()
    }

    /// Resolves the binary without ever downloading or executing anything
    /// outside the paths the user explicitly controls.
    fn binary(&self) -> Result<PathBuf, AppError> {
        if let Ok(path) = std::env::var("SUDOKOOLER_BIN") {
            let path = PathBuf::from(path);
            if path.is_file() {
                return Ok(path);
            }
        }
        // Installed / portable builds keep the daemon beside the app, not in
        // the process working directory (a shortcut starts in System32).
        if let Ok(exe) = std::env::current_exe()
            && let Some(dir) = exe.parent()
        {
            for name in [
                "sudokooler.exe",
                "sudokooler",
                "nsh/sudokooler.exe",
                "nsh/sudokooler",
            ] {
                let path = dir.join(name);
                if path.is_file() {
                    return Ok(path);
                }
            }
        }
        let candidates = [
            "sudokooler/target/release/sudokooler.exe",
            "sudokooler/target/debug/sudokooler.exe",
            "sudokooler/target/release/sudokooler",
            "sudokooler/target/debug/sudokooler",
            "target/release/sudokooler.exe",
            "sudokooler.exe",
            "sudokooler",
        ];
        for candidate in candidates {
            let path = Path::new(candidate);
            if path.is_file() {
                return Ok(path.to_path_buf());
            }
        }
        // The Scheduled Task starts the app outside the repo directory, so
        // relative paths above are not enough for a dev build.
        let absolute = concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/sudokooler/target/debug/sudokooler.exe"
        );
        if Path::new(absolute).is_file() {
            return Ok(PathBuf::from(absolute));
        }
        let absolute_linux = concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/sudokooler/target/debug/sudokooler"
        );
        if Path::new(absolute_linux).is_file() {
            return Ok(PathBuf::from(absolute_linux));
        }
        let absolute_release_linux = concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/sudokooler/target/release/sudokooler"
        );
        if Path::new(absolute_release_linux).is_file() {
            return Ok(PathBuf::from(absolute_release_linux));
        }
        Err(AppError::new(
            ErrorKind::Unavailable,
            "sudokooler binary not found; build the submodule or set SUDOKOOLER_BIN",
        ))
    }

    fn invoke(&self, args: &[&str]) -> Result<String, AppError> {
        let binary = self.binary()?;
        let mut command = Command::new(binary);
        #[cfg(windows)]
        command.creation_flags(CREATE_NO_WINDOW);
        command.args(args);
        if let Some(config) = &self.config_path {
            command.args(["--config", config]);
        }
        let output = command.output().map_err(|error| {
            AppError::new(
                ErrorKind::Unavailable,
                format!("cannot start sudokooler: {error}"),
            )
        })?;
        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr).trim().to_owned();
            let message = if stderr.is_empty() {
                format!("sudokooler exited with {}", output.status)
            } else {
                stderr
            };
            return Err(AppError::new(ErrorKind::Unavailable, message));
        }
        // Keep the daemon's stderr warnings (e.g. why sensors are unavailable)
        // alongside stdout so the UI can surface the real cause.
        let mut combined = String::from_utf8_lossy(&output.stdout).into_owned();
        let stderr = String::from_utf8_lossy(&output.stderr);
        let notice = stderr
            .lines()
            .find(|line| line.contains("[sensors]"))
            .map(str::to_owned);
        *self.notice.borrow_mut() = notice;
        if !stderr.trim().is_empty() {
            combined.push_str(&stderr);
        }
        Ok(combined)
    }

    fn telemetry_line(&self) -> Result<String, AppError> {
        let stdout = self.invoke(&["--dry-run", "--count", "1", "--interval-ms", "100"])?;
        stdout
            .lines()
            .rev()
            .find(|line| line.contains("temp=") && line.contains("power="))
            .map(str::to_owned)
            .ok_or_else(|| {
                AppError::new(
                    ErrorKind::InvalidSample,
                    "sudokooler --dry-run produced no telemetry line",
                )
            })
    }
}

impl DeviceSource for CliDaemon {
    fn discover(&self) -> Vec<Device> {
        let Ok(stdout) = self.invoke(&["--list-devices"]) else {
            return default_profiles();
        };
        let mut devices = default_profiles();
        for device in &mut devices {
            let id = device.id();
            device.connected = stdout.lines().any(|line| line.contains(&id));
        }
        devices
    }
}

/// Extracts a numeric field written as `key<value><suffix>`.
fn field<'a>(line: &'a str, key: &str, suffix: &str) -> Option<&'a str> {
    let rest = line.split_once(key)?.1;
    Some(rest.split(suffix).next()?.trim())
}

impl TelemetrySource for CliDaemon {
    fn read(&self) -> Result<Telemetry, AppError> {
        let line = self.telemetry_line()?;
        let temperature = field(&line, "temp=", "C")
            .and_then(|value| value.parse::<f32>().ok())
            .ok_or_else(|| AppError::new(ErrorKind::InvalidSample, "invalid temperature field"))?;
        let usage = field(&line, "load=", "%")
            .and_then(|value| value.parse::<u8>().ok())
            .ok_or_else(|| AppError::new(ErrorKind::InvalidSample, "invalid load field"))?;
        let power = field(&line, "power=", "W")
            .and_then(|value| value.parse::<u16>().ok())
            .ok_or_else(|| AppError::new(ErrorKind::InvalidSample, "invalid power field"))?;
        let frequency = field(&line, "freq=", "MHz")
            .and_then(|value| value.parse::<u16>().ok())
            .ok_or_else(|| AppError::new(ErrorKind::InvalidSample, "invalid frequency field"))?;

        Telemetry::new(temperature, usage, power, frequency).map_err(Into::into)
    }

    fn backend_name(&self) -> &'static str {
        "sudocooler CLI"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_the_daemon_dry_run_line() {
        let line = "[dry-run] temp=47.0C load=16% power=40W freq=5025MHz | 10680109";
        assert_eq!(field(line, "temp=", "C"), Some("47.0"));
        assert_eq!(field(line, "load=", "%"), Some("16"));
        assert_eq!(field(line, "power=", "W"), Some("40"));
        assert_eq!(field(line, "freq=", "MHz"), Some("5025"));
    }
}
