//! Windows logon autostart for sudocooler, stored in the per-user Run key.

use std::process::Command;

#[cfg(windows)]
use std::os::windows::process::CommandExt;
#[cfg(windows)]
const CREATE_NO_WINDOW: u32 = 0x0800_0000;

const KEY: &str = r"HKCU\Software\Microsoft\Windows\CurrentVersion\Run";
const NAME: &str = "sudocooler";

fn reg(args: &[&str]) -> Result<String, String> {
    let mut command = Command::new("reg.exe");
    #[cfg(windows)]
    command.creation_flags(CREATE_NO_WINDOW);
    let output = command
        .args(args)
        .output()
        .map_err(|error| format!("cannot run reg.exe: {error}"))?;
    if output.status.success() {
        Ok(String::from_utf8_lossy(&output.stdout).into_owned())
    } else {
        Err(String::from_utf8_lossy(&output.stderr).trim().to_owned())
    }
}

/// Registers the running executable to start at logon.
pub fn enable() -> Result<(), String> {
    let exe = std::env::current_exe()
        .map_err(|error| format!("cannot find executable: {error}"))?
        .to_string_lossy()
        .into_owned();
    let quoted = format!("\"{exe}\"");
    reg(&["add", KEY, "/v", NAME, "/t", "REG_SZ", "/d", &quoted, "/f"])?;
    Ok(())
}

/// Removes the logon entry.
pub fn disable() -> Result<(), String> {
    reg(&["delete", KEY, "/v", NAME, "/f"]).map(|_| ())
}

/// Whether the logon entry exists.
pub fn is_enabled() -> bool {
    reg(&["query", KEY, "/v", NAME]).is_ok()
}
