//! Logon autostart for sudocooler.
//!
//! Windows uses a Task Scheduler logon task at `RunLevel Highest`, the same
//! mechanism the installer registers, because PawnIO temperature/power need an
//! elevated process and a `Run` entry would show a UAC prompt at sign-in. The
//! task definition is created from XML (`schtasks /Create /XML`): `/TR` splits a
//! path containing spaces (`C:\Program Files\...` became `Command=C:\Program`
//! and failed with `0x80070002` in earlier builds).
//!
//! Linux uses an XDG autostart desktop entry in the current user's config.

#[cfg(windows)]
use std::os::windows::process::CommandExt;
#[cfg(windows)]
use std::process::{Command, Stdio};

/// `CREATE_NO_WINDOW`: schtasks.exe must not flash a console at the user.
#[cfg(windows)]
const CREATE_NO_WINDOW: u32 = 0x0800_0000;

#[cfg(windows)]
const TASK_NAME: &str = "sudocooler";

#[cfg(windows)]
fn tool(program: &str, args: &[&str]) -> std::io::Result<std::process::Output> {
    Command::new(program)
        .args(args)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .creation_flags(CREATE_NO_WINDOW)
        .output()
}

/// XML-escapes text placed in the task definition.
#[cfg(windows)]
fn xml_escape(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

/// The logon task: highest run level, `--minimized`, for this user.
#[cfg(windows)]
fn task_xml() -> Result<String, String> {
    let exe =
        std::env::current_exe().map_err(|error| format!("cannot find executable: {error}"))?;
    let user = std::env::var("USERNAME").map_err(|_| "USERNAME is not set".to_owned())?;
    let domain = std::env::var("USERDOMAIN").unwrap_or_default();
    let user_id = if domain.is_empty() {
        user
    } else {
        format!("{domain}\\{user}")
    };
    Ok(format!(
        r#"<?xml version="1.0" encoding="UTF-16"?>
<Task version="1.2" xmlns="http://schemas.microsoft.com/windows/2004/02/mit/task">
  <RegistrationInfo>
    <Description>sudocooler at sign-in</Description>
  </RegistrationInfo>
  <Triggers>
    <LogonTrigger>
      <Enabled>true</Enabled>
    </LogonTrigger>
  </Triggers>
  <Principals>
    <Principal id="Author">
      <UserId>{user}</UserId>
      <LogonType>InteractiveToken</LogonType>
      <RunLevel>HighestAvailable</RunLevel>
    </Principal>
  </Principals>
  <Settings>
    <MultipleInstancesPolicy>IgnoreNew</MultipleInstancesPolicy>
    <DisallowStartIfOnBatteries>false</DisallowStartIfOnBatteries>
    <StopIfGoingOnBatteries>false</StopIfGoingOnBatteries>
    <AllowHardTerminate>true</AllowHardTerminate>
    <StartWhenAvailable>true</StartWhenAvailable>
  </Settings>
  <Actions Context="Author">
    <Exec>
      <Command>{exe}</Command>
      <Arguments>--minimized</Arguments>
    </Exec>
  </Actions>
</Task>
"#,
        user = xml_escape(&user_id),
        exe = xml_escape(&exe.display().to_string()),
    ))
}

/// Writes the task XML as UTF-16 LE with a BOM, as Task Scheduler expects.
#[cfg(windows)]
fn write_task_file(xml: &str) -> Result<std::path::PathBuf, String> {
    let path = std::env::temp_dir().join(format!("sudocooler-task-{}.xml", std::process::id()));
    let mut bytes = vec![0xFF, 0xFE];
    for unit in xml.encode_utf16() {
        bytes.extend_from_slice(&unit.to_le_bytes());
    }
    std::fs::write(&path, bytes).map_err(|error| format!("cannot write the task XML: {error}"))?;
    Ok(path)
}

#[cfg(windows)]
fn delete_legacy_run_entry() {
    // 0.1.14 wrote a per-user Run value; it would start the app unelevated.
    let _ = tool(
        "reg.exe",
        &[
            "delete",
            r"HKCU\Software\Microsoft\Windows\CurrentVersion\Run",
            "/v",
            "sudocooler",
            "/f",
        ],
    );
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
        let xml = task_xml()?;
        let path = write_task_file(&xml)?;
        let path_text = path.display().to_string();
        let result = tool(
            "schtasks.exe",
            &["/Create", "/TN", TASK_NAME, "/XML", &path_text, "/F"],
        );
        let _ = std::fs::remove_file(&path);
        let output = result.map_err(|error| format!("cannot run schtasks.exe: {error}"))?;
        if !output.status.success() {
            return Err(format!(
                "Task Scheduler refused the task: {}",
                String::from_utf8_lossy(&output.stderr).trim()
            ));
        }
        delete_legacy_run_entry();
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
            "[Desktop Entry]\nType=Application\nName=sudocooler\nComment=Sudokoo CPU cooler companion\nExec=\"{exe}\" --minimized\nTerminal=false\nX-GNOME-Autostart-enabled=true\n"
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
        let _ = tool("schtasks.exe", &["/Delete", "/TN", TASK_NAME, "/F"]);
        delete_legacy_run_entry();
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

/// Whether the app will start at logon.
pub fn is_enabled() -> bool {
    #[cfg(windows)]
    {
        tool("schtasks.exe", &["/Query", "/TN", TASK_NAME])
            .map(|output| output.status.success())
            .unwrap_or(false)
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

/// Removes the per-user Run entry a 0.1.14 install may have written. No-op
/// when there is none; safe to call at startup.
pub fn migrate() {
    #[cfg(windows)]
    delete_legacy_run_entry();
}
