//! Windows logon autostart for sudocooler.
//!
//! A Scheduled Task is used instead of the Run key because PawnIO needs an
//! elevated process to read CPU temperature and package power.

use std::process::Command;

#[cfg(windows)]
use std::os::windows::process::CommandExt;
#[cfg(windows)]
const CREATE_NO_WINDOW: u32 = 0x0800_0000;

const TASK: &str = "sudocooler";

fn schtasks(args: &[&str]) -> Result<String, String> {
    let mut command = Command::new("schtasks.exe");
    #[cfg(windows)]
    command.creation_flags(CREATE_NO_WINDOW);
    let output = command
        .args(args)
        .output()
        .map_err(|error| format!("cannot run schtasks.exe: {error}"))?;
    if output.status.success() {
        Ok(String::from_utf8_lossy(&output.stdout).into_owned())
    } else {
        Err(String::from_utf8_lossy(&output.stderr).trim().to_owned())
    }
}

/// Registers the running executable to start elevated at logon.
pub fn enable() -> Result<(), String> {
    let exe = std::env::current_exe()
        .map_err(|error| format!("cannot find executable: {error}"))?
        .to_string_lossy()
        .into_owned();
    let command = format!("\"{exe}\"");
    schtasks(&[
        "/Create", "/TN", TASK, "/TR", &command, "/SC", "ONLOGON", "/RL", "HIGHEST", "/F",
    ])?;
    Ok(())
}

/// Removes the logon task.
pub fn disable() -> Result<(), String> {
    schtasks(&["/Delete", "/TN", TASK, "/F"]).map(|_| ())
}

/// Whether the logon task exists.
pub fn is_enabled() -> bool {
    schtasks(&["/Query", "/TN", TASK]).is_ok()
}
