//! Adapter that drives the `sudokooler` binary shipped as a git submodule.
//!
//! It uses the daemon's stable CLI instead of linking its internals: the
//! submodule stays a standalone project with its own safety guardrails, and
//! this UI keeps working if the daemon is built elsewhere.
//!
//! One long-lived `sudokooler --dry-run` process feeds every sample. Spawning a
//! process per reading (as earlier builds did) left 80+ short-lived processes
//! behind in the first minute and paid the daemon's PawnIO/HID start-up cost on
//! every tick. The process is held in a job object on Windows and asks for
//! `SIGTERM` when its parent dies on Linux, so it cannot outlive the app.

use std::cell::RefCell;
use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};
use std::process::{Child, ChildStderr, ChildStdout, Command, Stdio};
use std::sync::{Mutex, OnceLock};

#[cfg(windows)]
use std::os::windows::process::CommandExt;
#[cfg(windows)]
const CREATE_NO_WINDOW: u32 = 0x0800_0000;

use crate::application::ports::{DeviceSource, TelemetrySource};
use crate::application::{AppError, ErrorKind};
use crate::domain::device::{Device, default_profiles};
use crate::domain::telemetry::Telemetry;

/// Runs the daemon's one-shot commands and keeps one sampling process alive.
pub struct CliDaemon {
    config_path: Option<String>,
    interval_ms: u32,
    /// Sensor warning captured from the last sample's stderr (if any).
    notice: RefCell<Option<String>>,
}

impl CliDaemon {
    pub fn new(config_path: Option<String>, interval_ms: u32) -> Self {
        Self {
            config_path,
            interval_ms,
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

    /// Runs a one-shot daemon command (device enumeration).
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
        Ok(String::from_utf8_lossy(&output.stdout).into_owned())
    }

    /// The next telemetry line from the long-lived sampling process.
    fn telemetry_line(&self) -> Result<String, AppError> {
        let binary = self.binary()?;
        let mut stream = stream_slot().lock().map_err(|error| {
            AppError::new(ErrorKind::Unavailable, format!("sampler lock: {error}"))
        })?;

        let stale = stream.as_ref().is_some_and(|running| {
            running.binary != binary
                || running.interval_ms != self.interval_ms
                || running.config_path != self.config_path
        });
        if stale {
            // Dropping the stream kills its process and closes the pipes.
            *stream = None;
        }
        if stream.is_none() {
            let mut started =
                Stream::spawn(&binary, self.config_path.as_deref(), self.interval_ms)?;
            let line = started.next_telemetry_line()?;
            *stream = Some(started);
            *self.notice.borrow_mut() = take_notice();
            return Ok(line);
        }

        let running = stream.as_mut().expect("checked above");
        let line = match running.next_telemetry_line() {
            Ok(line) => line,
            Err(error) => {
                // The process is gone or wedged; the next tick starts a new one.
                *stream = None;
                return Err(error);
            }
        };
        *self.notice.borrow_mut() = take_notice();
        Ok(line)
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

// ---------------------------------------------------------------- sampler --

/// The one sampling process, restarted when the interval, config or binary
/// changes and killed with the app.
struct Stream {
    child: Child,
    stdout: BufReader<ChildStdout>,
    binary: PathBuf,
    config_path: Option<String>,
    interval_ms: u32,
    /// Windows: kills the child if this process dies without cleaning up.
    #[cfg(windows)]
    job: Option<Job>,
}

impl Stream {
    fn spawn(binary: &Path, config: Option<&str>, interval_ms: u32) -> Result<Self, AppError> {
        let interval = interval_ms.clamp(100, 60_000);
        let mut command = Command::new(binary);
        command
            .args([
                "--dry-run",
                "--count",
                "0",
                "--interval-ms",
                &interval.to_string(),
            ])
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        if let Some(config) = config {
            command.args(["--config", config]);
        }
        #[cfg(windows)]
        command.creation_flags(CREATE_NO_WINDOW);
        #[cfg(target_os = "linux")]
        {
            use std::os::unix::process::CommandExt;
            // The kernel signals the child if this process dies abruptly.
            // SAFETY: `prctl` is async-signal-safe and has no preconditions.
            unsafe {
                command.pre_exec(|| {
                    libc::prctl(libc::PR_SET_PDEATHSIG, libc::SIGTERM);
                    Ok(())
                });
            }
        }

        let mut child = command.spawn().map_err(|error| {
            AppError::new(
                ErrorKind::Unavailable,
                format!("cannot start sudokooler: {error}"),
            )
        })?;
        let stdout = child.stdout.take().ok_or_else(|| {
            AppError::new(ErrorKind::Unavailable, "sudokooler stdout unavailable")
        })?;
        if let Some(stderr) = child.stderr.take() {
            std::thread::Builder::new()
                .name("sudokooler-log".to_owned())
                .spawn(move || read_stderr(stderr))
                .ok();
        }
        #[cfg(windows)]
        let job = Job::new(&child);

        Ok(Self {
            child,
            stdout: BufReader::new(stdout),
            binary: binary.to_path_buf(),
            config_path: config.map(str::to_owned),
            interval_ms: interval,
            #[cfg(windows)]
            job,
        })
    }

    /// The next line carrying telemetry; the daemon's banner is skipped.
    fn next_telemetry_line(&mut self) -> Result<String, AppError> {
        let mut line = String::new();
        loop {
            line.clear();
            let read = self.stdout.read_line(&mut line).map_err(|error| {
                AppError::new(
                    ErrorKind::Unavailable,
                    format!("cannot read sudokooler output: {error}"),
                )
            })?;
            if read == 0 {
                return Err(AppError::new(
                    ErrorKind::Unavailable,
                    "sudokooler stopped; restarting it",
                ));
            }
            if line.contains("temp=") && line.contains("power=") {
                return Ok(line.trim().to_owned());
            }
        }
    }
}

impl Drop for Stream {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
        #[cfg(windows)]
        drop(self.job.take());
    }
}

/// Last `[sensors]` warning the daemon wrote on stderr.
static NOTICE: Mutex<Option<String>> = Mutex::new(None);

fn read_stderr(stderr: ChildStderr) {
    let reader = BufReader::new(stderr);
    for line in reader.lines().map_while(Result::ok) {
        if let Some(notice) = line.strip_prefix("[sensors] ")
            && let Ok(mut slot) = NOTICE.lock()
        {
            *slot = Some(notice.trim().to_owned());
        }
    }
}

fn take_notice() -> Option<String> {
    NOTICE.lock().ok().and_then(|mut slot| slot.take())
}

fn stream_slot() -> &'static Mutex<Option<Stream>> {
    static SLOT: OnceLock<Mutex<Option<Stream>>> = OnceLock::new();
    SLOT.get_or_init(|| Mutex::new(None))
}

/// Stops the sampling process (app paused, settings changed, app quitting).
pub fn shutdown() {
    if let Ok(mut stream) = stream_slot().lock() {
        // Dropping kills the process and releases its pipes.
        *stream = None;
    }
    if let Ok(mut notice) = NOTICE.lock() {
        *notice = None;
    }
}

#[cfg(windows)]
struct Job(usize);

#[cfg(windows)]
impl Job {
    /// A job whose close kills its process: an app that crashes or is killed
    /// from Task Manager cannot leave a sampling daemon behind.
    fn new(child: &Child) -> Option<Self> {
        use std::os::windows::io::AsRawHandle;

        use windows_sys::Win32::System::JobObjects::{
            AssignProcessToJobObject, CreateJobObjectW, JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE,
            JOBOBJECT_EXTENDED_LIMIT_INFORMATION, JobObjectExtendedLimitInformation,
            SetInformationJobObject,
        };

        // SAFETY: standard job-object calls; every handle is checked, and the
        // job is kept open for the process's life (its close kills the child).
        unsafe {
            let job = CreateJobObjectW(std::ptr::null(), std::ptr::null());
            if job.is_null() {
                return None;
            }
            let mut info: JOBOBJECT_EXTENDED_LIMIT_INFORMATION = std::mem::zeroed();
            info.BasicLimitInformation.LimitFlags = JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE;
            let size = std::mem::size_of::<JOBOBJECT_EXTENDED_LIMIT_INFORMATION>() as u32;
            if SetInformationJobObject(
                job,
                JobObjectExtendedLimitInformation,
                &info as *const _ as *const std::ffi::c_void,
                size,
            ) == 0
                || AssignProcessToJobObject(job, child.as_raw_handle().cast()) == 0
            {
                let _ = windows_sys::Win32::Foundation::CloseHandle(job);
                return None;
            }
            Some(Self(job as usize))
        }
    }
}

#[cfg(windows)]
impl Drop for Job {
    fn drop(&mut self) {
        // Closing the last handle to the job kills the process assigned to it.
        // SAFETY: this handle came from `CreateJobObjectW` and is closed once.
        unsafe {
            windows_sys::Win32::Foundation::CloseHandle(
                self.0 as windows_sys::Win32::Foundation::HANDLE,
            )
        };
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
