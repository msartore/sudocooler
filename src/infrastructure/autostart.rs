//! Logon autostart for sudocooler.
//!
//! Windows uses an elevated Scheduled Task (PawnIO needs elevation). Linux
//! uses an XDG autostart desktop entry in the current user's config.

#[cfg(windows)]
use std::process::Command;

#[cfg(windows)]
use std::os::windows::process::CommandExt;
#[cfg(windows)]
const CREATE_NO_WINDOW: u32 = 0x0800_0000;

#[cfg(windows)]
const KEY: &str = r"HKCU\Software\Microsoft\Windows\CurrentVersion\Run";
#[cfg(windows)]
const NAME: &str = "sudocooler";

#[cfg(windows)]
fn reg(args: &[&str]) -> Result<String, String> {
    let mut command = Command::new("reg.exe");
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

#[cfg(target_os = "linux")]
fn desktop_entry() -> Result<std::path::PathBuf, String> {
    let home = std::env::var("HOME").map_err(|_| "HOME is not set".to_owned())?;
    Ok(std::path::PathBuf::from(home)
        .join(".config/autostart")
        .join("sudocooler.desktop"))
}

/// Registers the running executable to start at logon.
pub fn enable() -> Result<(), String> {
    let exe = std::env::current_exe()
        .map_err(|error| format!("cannot find executable: {error}"))?
        .to_string_lossy()
        .into_owned();

    #[cfg(windows)]
    {
        let command = format!("\"{exe}\"");
        reg(&["add", KEY, "/v", NAME, "/t", "REG_SZ", "/d", &command, "/f"])?;
    }

    #[cfg(target_os = "linux")]
    {
        let path = desktop_entry()?;
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).map_err(|error| error.to_string())?;
        }
        let contents = format!(
            "[Desktop Entry]\nType=Application\nName=sudocooler\nComment=Sudokoo CPU cooler companion\nExec={exe}\nTerminal=false\nX-GNOME-Autostart-enabled=true\n"
        );
        std::fs::write(&path, contents).map_err(|error| error.to_string())?;
    }

    #[cfg(not(any(windows, target_os = "linux")))]
    let _ = exe;

    Ok(())
}

/// Removes the logon entry.
pub fn disable() -> Result<(), String> {
    #[cfg(windows)]
    reg(&["delete", KEY, "/v", NAME, "/f"]).map(|_| ())?;

    #[cfg(target_os = "linux")]
    {
        let path = desktop_entry()?;
        if path.exists() {
            std::fs::remove_file(&path).map_err(|error| error.to_string())?;
        }
    }

    Ok(())
}

/// Whether the logon entry exists.
pub fn is_enabled() -> bool {
    #[cfg(windows)]
    {
        reg(&["query", KEY, "/v", NAME]).is_ok()
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
