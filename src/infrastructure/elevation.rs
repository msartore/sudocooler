//! Privilege state, and an explicit request for elevation.
//!
//! The app starts unelevated: lighting and the dashboard never need
//! administrator rights, and a sign-in launch must not show a UAC prompt.
//! PawnIO's driver does need them for CPU temperature and package power, so the
//! user consents in the app and Windows shows its own prompt.
//!
//! Windows raises it through `ShellExecuteEx(..., "runas")`. The calling
//! instance exits once the elevated one has started; because the app is
//! single-instance, the launch is handed to a hidden helper that waits for this
//! PID to disappear first (Windows blocks `WM_COPYDATA` between integrity
//! levels, so a copy started while this one holds the mutex would only forward
//! its arguments and exit).

#[derive(Debug, Clone)]
pub struct Elevation {
    /// True when the current process already holds an elevated token (or euid 0).
    pub elevated: bool,
    pub can_request: bool,
    /// Human-readable reason, shown verbatim in the UI.
    pub detail: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Outcome {
    /// The elevated instance was started; the caller should exit.
    Launched,
    Declined,
    Unsupported,
}

/// Current privilege state, without side effects.
pub fn state() -> Elevation {
    if is_elevated() {
        return Elevation {
            elevated: true,
            can_request: false,
            detail: imp::ELEVATED_DETAIL.to_string(),
        };
    }
    Elevation {
        elevated: false,
        can_request: imp::CAN_REQUEST,
        detail: imp::UNELEVATED_DETAIL.to_string(),
    }
}

pub fn is_elevated() -> bool {
    imp::is_elevated()
}

/// A copy of this app is running as administrator and NSH's single-instance
/// mutex is out of reach across the integrity boundary, so a standard-user
/// launch starts a second instance instead of forwarding. Two instances must
/// never run (both poll and write the same cooler): focus the elevated window
/// and exit. Windows-only; `false` when no other copy exists.
pub fn focus_running_instance() -> bool {
    #[cfg(target_os = "windows")]
    {
        imp::focus_running_instance()
    }
    #[cfg(not(target_os = "windows"))]
    {
        false
    }
}

pub fn request() -> Result<Outcome, String> {
    imp::request()
}

#[cfg(target_os = "windows")]
mod imp {
    use std::ffi::{OsStr, c_void};
    use std::mem::{size_of, zeroed};
    use std::os::windows::ffi::OsStrExt;

    use windows_sys::Win32::Foundation::{CloseHandle, GetLastError, HANDLE};
    use windows_sys::Win32::Security::{GetTokenInformation, TOKEN_ELEVATION, TokenElevation};
    use windows_sys::Win32::System::Threading::{GetCurrentProcess, OpenProcessToken};
    use windows_sys::Win32::UI::Shell::{
        SEE_MASK_FLAG_NO_UI, SEE_MASK_NOCLOSEPROCESS, SHELLEXECUTEINFOW, ShellExecuteExW,
    };
    use windows_sys::Win32::UI::WindowsAndMessaging::SW_HIDE;

    use super::Outcome;

    /// `ERROR_CANCELLED` — the user dismissed the UAC prompt.
    const ERROR_CANCELLED: u32 = 1223;

    pub(super) const CAN_REQUEST: bool = true;
    pub(super) const ELEVATED_DETAIL: &str = "Running with administrator rights.";
    pub(super) const UNELEVATED_DETAIL: &str = "Running as a standard user. PawnIO's driver is out of reach, so \
         temperature and power read 0. Restarting as administrator asks Windows for consent once.";

    pub(super) fn is_elevated() -> bool {
        // SAFETY: every handle is released before returning, and `elevation` is
        // a live local of exactly the size passed to GetTokenInformation.
        unsafe {
            let mut token: HANDLE = std::ptr::null_mut();
            if OpenProcessToken(
                GetCurrentProcess(),
                0x0008, /* TOKEN_QUERY */
                &mut token,
            ) == 0
            {
                return false;
            }
            let mut elevation: TOKEN_ELEVATION = zeroed();
            let mut returned = 0u32;
            let ok = GetTokenInformation(
                token,
                TokenElevation,
                &mut elevation as *mut TOKEN_ELEVATION as *mut c_void,
                size_of::<TOKEN_ELEVATION>() as u32,
                &mut returned,
            );
            CloseHandle(token);
            ok != 0 && elevation.TokenIsElevated != 0
        }
    }

    pub(super) fn focus_running_instance() -> bool {
        use windows_sys::Win32::Foundation::{HWND, LPARAM, TRUE};
        use windows_sys::Win32::UI::WindowsAndMessaging::{
            EnumWindows, GetWindowTextW, GetWindowThreadProcessId, SW_RESTORE, SW_SHOW,
            SetForegroundWindow, ShowWindow,
        };

        // windows-sys 0.59 spells `BOOL` as `i32`.
        type Bool = i32;

        struct Search {
            pid: u32,
            found: HWND,
            /// This process's own window exists (a real app, not a headless test).
            own: bool,
        }

        unsafe extern "system" fn collect(hwnd: HWND, lparam: LPARAM) -> Bool {
            // SAFETY: `lparam` is the `&mut Search` passed to `EnumWindows`,
            // alive for the whole call.
            let search = unsafe { &mut *(lparam as *mut Search) };
            let mut pid = 0u32;
            // SAFETY: a valid window handle from the enumeration.
            unsafe { GetWindowThreadProcessId(hwnd, &mut pid) };
            let mut title = [0u16; 64];
            // SAFETY: the buffer is sized and lives for the call.
            let len = unsafe { GetWindowTextW(hwnd, title.as_mut_ptr(), title.len() as i32) };
            let title = String::from_utf16_lossy(&title[..len.max(0) as usize]);
            if pid == search.pid {
                // The runner creates this window before the app loads. A
                // headless test has none, and must not touch the desktop.
                if title == "sudocooler" {
                    search.own = true;
                }
                return TRUE;
            }
            if title == "sudocooler" && search.found.is_null() {
                search.found = hwnd;
                // Keep going: this process's own window may come later in the
                // enumeration order, and `own` must be known before deciding.
            }
            TRUE
        }

        let mut search = Search {
            pid: std::process::id(),
            found: std::ptr::null_mut(),
            own: false,
        };
        // SAFETY: `collect` only touches `search`, which outlives the call.
        unsafe { EnumWindows(Some(collect), &mut search as *mut Search as LPARAM) };
        if !search.own || search.found.is_null() {
            return false;
        }
        // SAFETY: the handle came from the enumeration, so it is a live
        // top-level window; one that closed meanwhile ignores the requests.
        unsafe {
            ShowWindow(search.found, SW_SHOW);
            ShowWindow(search.found, SW_RESTORE);
            SetForegroundWindow(search.found);
        }
        // The other copy owns the cooler. This one never shows a window.
        std::process::exit(0);
    }

    pub(super) fn request() -> Result<Outcome, String> {
        let exe = std::env::current_exe()
            .map_err(|error| format!("cannot locate the executable: {error}"))?;
        let cwd = std::env::current_dir().ok();

        let parameters = std::env::args_os()
            .skip(1)
            .filter(|arg| arg != OsStr::new("--minimized"))
            .map(|arg| quote(&arg))
            .collect::<Vec<_>>()
            .join(" ");

        // The app is single-instance: a copy started before this one exits is
        // forwarded here and exits, and Windows blocks that handoff across the
        // integrity boundary. A hidden helper waits for this PID, then raises
        // the prompt so the elevated copy starts as the only instance.
        let waiting = format!(
            "$p = {pid}; $deadline = (Get-Date).AddSeconds(20); while ((Get-Process -Id $p -ErrorAction SilentlyContinue) -and (Get-Date) -lt $deadline) {{ Start-Sleep -Milliseconds 200 }}; try {{ Start-Process -FilePath '{exe}' -ArgumentList {args} -Verb RunAs -ErrorAction Stop }} catch {{ Start-Process -FilePath '{exe}' -ArgumentList {args} }}",
            pid = std::process::id(),
            exe = exe.display().to_string().replace('\'', "''"),
            args = if parameters.is_empty() {
                "@()".to_string()
            } else {
                format!("@('{}')", parameters.replace('\'', "''"))
            },
        );
        let helper = format!(
            "-NoProfile -NonInteractive -WindowStyle Hidden -Command \"{}\"",
            waiting.replace('"', "`\"")
        );

        let verb = wide("open");
        let file = wide("powershell.exe");
        let params = wide(&helper);
        let dir = cwd.as_ref().map(wide);

        // SAFETY: all wide buffers outlive the call; the struct is fully
        // initialised apart from the union, which is zeroed.
        unsafe {
            let mut info: SHELLEXECUTEINFOW = zeroed();
            info.cbSize = size_of::<SHELLEXECUTEINFOW>() as u32;
            info.fMask = SEE_MASK_NOCLOSEPROCESS | SEE_MASK_FLAG_NO_UI;
            info.lpVerb = verb.as_ptr();
            info.lpFile = file.as_ptr();
            info.lpParameters = params.as_ptr();
            info.lpDirectory = dir.as_ref().map_or(std::ptr::null(), |d| d.as_ptr());
            info.nShow = SW_HIDE;

            if ShellExecuteExW(&mut info) == 0 {
                let err = GetLastError();
                if err == ERROR_CANCELLED {
                    return Ok(Outcome::Declined);
                }
                return Err(format!(
                    "elevation request failed: {}",
                    std::io::Error::from_raw_os_error(err as i32)
                ));
            }
            if !info.hProcess.is_null() {
                CloseHandle(info.hProcess);
            }
        }
        Ok(Outcome::Launched)
    }

    fn wide(s: impl AsRef<OsStr>) -> Vec<u16> {
        s.as_ref().encode_wide().chain(std::iter::once(0)).collect()
    }

    /// Quotes one argument for `CommandLineToArgvW` parsing.
    fn quote(arg: &OsStr) -> String {
        let s = arg.to_string_lossy();
        if !s.is_empty() && !s.contains([' ', '\t', '\n', '"']) {
            return s.into_owned();
        }
        let mut out = String::with_capacity(s.len() + 2);
        out.push('"');
        let mut backslashes = 0usize;
        for ch in s.chars() {
            match ch {
                '\\' => backslashes += 1,
                '"' => {
                    for _ in 0..backslashes * 2 + 1 {
                        out.push('\\');
                    }
                    out.push('"');
                    backslashes = 0;
                }
                _ => {
                    for _ in 0..backslashes {
                        out.push('\\');
                    }
                    backslashes = 0;
                    out.push(ch);
                }
            }
        }
        for _ in 0..backslashes * 2 {
            out.push('\\');
        }
        out.push('"');
        out
    }

    #[cfg(test)]
    mod tests {
        use super::quote;
        use std::ffi::OsStr;

        #[test]
        fn plain_arguments_are_not_quoted() {
            assert_eq!(quote(OsStr::new("--debug")), "--debug");
        }

        #[test]
        fn spaces_force_quotes() {
            assert_eq!(quote(OsStr::new("a b")), "\"a b\"");
        }

        #[test]
        fn quotes_are_escaped() {
            assert_eq!(quote(OsStr::new("say \"hi\"")), "\"say \\\"hi\\\"\"");
        }
    }
}

#[cfg(target_os = "linux")]
mod imp {
    use super::Outcome;

    pub(super) const CAN_REQUEST: bool = false;
    pub(super) const ELEVATED_DETAIL: &str = "Running as root.";
    pub(super) const UNELEVATED_DETAIL: &str = "CPU telemetry is read from sysfs/hwmon, which needs no elevation. \
         PawnIO is Windows-only.";

    pub(super) fn is_elevated() -> bool {
        // SAFETY: geteuid has no preconditions and cannot fail.
        unsafe { libc::geteuid() == 0 }
    }

    pub(super) fn request() -> Result<Outcome, String> {
        Ok(Outcome::Unsupported)
    }
}

#[cfg(not(any(target_os = "windows", target_os = "linux")))]
mod imp {
    use super::Outcome;

    pub(super) const CAN_REQUEST: bool = false;
    pub(super) const ELEVATED_DETAIL: &str = "Privileges are not tracked on this platform.";
    pub(super) const UNELEVATED_DETAIL: &str =
        "Hardware access is unavailable on this platform, so elevation would not help.";

    pub(super) fn is_elevated() -> bool {
        false
    }

    pub(super) fn request() -> Result<Outcome, String> {
        Ok(Outcome::Unsupported)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn state_never_claims_both_elevated_and_requestable() {
        let state = state();
        assert!(!(state.elevated && state.can_request));
        assert!(!state.detail.is_empty());
    }

    #[test]
    fn detection_is_consistent_between_calls() {
        assert_eq!(is_elevated(), is_elevated());
    }
}
