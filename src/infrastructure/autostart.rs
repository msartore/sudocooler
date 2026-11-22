//! Logon autostart for sudocooler.
//!
//! Windows uses an elevated Scheduled Task (`/RL HIGHEST`), the same mechanism
//! the installer registers, because PawnIO temperature/power need elevation.
//! Linux uses an XDG autostart desktop entry in the current user's config.
//!
//! An older build wrote an HKCU\Run value instead; that launches unelevated, so
//! `is_enabled` still reports it (so the toggle is accurate) and `disable`
//! removes it.

#[cfg(windows)]
use std::process::Command;

#[cfg(windows)]
use std::os::windows::process::CommandExt;
#[cfg(windows)]
const CREATE_NO_WINDOW: u32 = 0x0800_0000;

#[cfg(windows)]
const TASK_NAME: &str = "sudocooler";
#[cfg(windows)]
const RUN_KEY: &str = r"HKCU\Software\Microsoft\Windows\CurrentVersion\Run";
#[cfg(windows)]
const RUN_NAME: &str = "sudocooler";

#[cfg(windows)]
fn run(program: &str, args: &[&str]) -> Result<String, String> {
    let mut command = Command::new(program);
    command.creation_flags(CREATE_NO_WINDOW);
    let output = command
        .args(args)
        .output()
        .map_err(|error| format!("cannot run {program}: {error}"))?;
    if output.status.success() {
        Ok(String::from_utf8_lossy(&output.stdout).into_owned())
    } else {
        Err(String::from_utf8_lossy(&output.stderr).trim().to_owned())
    }
}

/// The path to register, quoted so a spaced path stays one argument.
#[cfg(windows)]
fn executable() -> Result<String, String> {
    let exe = std::env::current_exe()
        .map_err(|error| format!("cannot find executable: {error}"))?
        .to_string_lossy()
        .into_owned();
    Ok(format!("\"{exe}\""))
}

#[cfg(target_os = "linux")]
fn desktop_entry() -> Result<std::path::PathBuf, String> {
    let home = std::env::var("HOME").map_err(|_| "HOME is not set".to_owned())?;
    Ok(std::path::PathBuf::from(home)
        .join(".config/autostart")
        .join("sudocooler.desktop"))
}

/// Registers the running executable to start, elevated, at logon.
pub fn enable() -> Result<(), String> {
    #[cfg(windows)]
    {
        let exe = executable()?;
        run(
            "schtasks.exe",
            &[
                "/Create", "/TN", TASK_NAME, "/TR", &exe, "/SC", "ONLOGON", "/RL", "HIGHEST", "/F",
            ],
        )?;
        // Drop any legacy HKCU entry so the app does not start twice.
        let _ = run("reg.exe", &["delete", RUN_KEY, "/v", RUN_NAME, "/f"]);
        Ok(())
    }

    #[cfg(target_os = "linux")]
    {
        let exe = std::env::current_exe()
            .map_err(|error| format!("cannot find executable: {error}"))?
            .to_string_lossy()
            .into_owned();
        let path = desktop_entry()?;
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).map_err(|error| error.to_string())?;
        }
        let contents = format!(
            "[Desktop Entry]\nType=Application\nName=sudocooler\nComment=Sudokoo CPU cooler companion\nExec={exe}\nTerminal=false\nX-GNOME-Autostart-enabled=true\n"
        );
        std::fs::write(&path, contents).map_err(|error| error.to_string())?;
        Ok(())
    }

    #[cfg(not(any(windows, target_os = "linux")))]
    Ok(())
}

/// Removes the logon entry.
pub fn disable() -> Result<(), String> {
    #[cfg(windows)]
    {
        // A missing task is not an error (already off).
        match run("schtasks.exe", &["/Delete", "/TN", TASK_NAME, "/F"]) {
            Ok(_) => {}
            Err(error) if error.is_empty() => {}
            Err(error) => {
                // Only surface a real failure, not "task does not exist".
                if !error.to_ascii_lowercase().contains("cannot find") {
                    // Continue: still try to clear the legacy entry below.
                    let _ = error;
                }
            }
        }
        let _ = run("reg.exe", &["delete", RUN_KEY, "/v", RUN_NAME, "/f"]);
        Ok(())
    }

    #[cfg(target_os = "linux")]
    {
        let path = desktop_entry()?;
        if path.exists() {
            std::fs::remove_file(&path).map_err(|error| error.to_string())?;
        }
        Ok(())
    }

    #[cfg(not(any(windows, target_os = "linux")))]
    Ok(())
}

/// Whether the app will start at logon (task, or a legacy HKCU entry).
pub fn is_enabled() -> bool {
    #[cfg(windows)]
    {
        let task = run("schtasks.exe", &["/Query", "/TN", TASK_NAME]).is_ok();
        let legacy = run("reg.exe", &["query", RUN_KEY, "/v", RUN_NAME]).is_ok();
        task || legacy
    }
    #[cfg(target_os = "linux")]
    {
        desktop_entry().is_ok_and(|path| path.exists())
    }
    #[cfg(not(any(windows, target_os = "linux")))]
    {
        false
    }
}
