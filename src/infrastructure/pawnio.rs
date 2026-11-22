//! PawnIO availability probe, modelled on OpenColorHub's implementation.
//!
//! The daemon reads CPU temperature and package power through PawnIO's signed
//! driver when it is elevated. The app itself only *probes* the driver so it
//! can tell "not installed" (offer to set it up) apart from "installed but not
//! elevated" (ask to restart as administrator) instead of guessing.
//!
//! It loads the official `PawnIOLib.dll` and calls `pawnio_open` / `pawnio_close`
//! (and `pawnio_version` when present), so the classification matches the
//! library's own HRESULTs rather than re-implementing the device protocol.

use crate::application::PawnIoStatus;

/// Where PawnIO installs its user-mode library; the bare name covers the
/// default search order some deployments register.
#[cfg(windows)]
const DLL_CANDIDATES: [&str; 4] = [
    r"C:\Program Files\PawnIO\PawnIOLib.dll",
    r"C:\Program Files (x86)\PawnIO\PawnIOLib.dll",
    r"C:\Windows\System32\PawnIOLib.dll",
    "PawnIOLib.dll",
];

/// `0x80070005` — the device object rejects unelevated callers.
#[cfg(windows)]
const E_ACCESS_DENIED: u32 = 0x8007_0005;

#[cfg(windows)]
fn describe(hr: i32) -> String {
    let unsigned = hr as u32;
    if unsigned == E_ACCESS_DENIED {
        return "access denied — PawnIO requires administrator rights".to_owned();
    }
    if (unsigned & 0xFFFF_0000) == 0x8007_0000 {
        let code = unsigned & 0xFFFF;
        let text = std::io::Error::from_raw_os_error(code as i32).to_string();
        return format!("{text} (0x{unsigned:08X})");
    }
    format!("HRESULT 0x{unsigned:08X}")
}

/// Probes the driver without keeping it open.
#[cfg(windows)]
pub fn status() -> PawnIoStatus {
    use windows_sys::Win32::Foundation::HANDLE;
    use windows_sys::Win32::System::LibraryLoader::{GetProcAddress, LoadLibraryW};

    type Hr = i32;
    type PawnioVersion = unsafe extern "system" fn(*mut u32) -> Hr;
    type PawnioOpen = unsafe extern "system" fn(*mut HANDLE) -> Hr;
    type PawnioClose = unsafe extern "system" fn(HANDLE) -> Hr;

    let mut library: *mut core::ffi::c_void = std::ptr::null_mut();
    for path in DLL_CANDIDATES {
        let wide: Vec<u16> = path.encode_utf16().chain(std::iter::once(0)).collect();
        // SAFETY: `wide` is NUL-terminated and outlives the call.
        let handle = unsafe { LoadLibraryW(wide.as_ptr()) };
        if !handle.is_null() {
            library = handle;
            break;
        }
    }
    if library.is_null() {
        return PawnIoStatus::NotInstalled;
    }

    let symbol = |name: &str| -> windows_sys::Win32::Foundation::FARPROC {
        let c: Vec<u8> = name.bytes().chain(std::iter::once(0)).collect();
        // SAFETY: `library` is a live module handle and `c` is NUL-terminated.
        unsafe { GetProcAddress(library, c.as_ptr()) }
    };

    let (Some(open_addr), Some(close_addr)) = (symbol("pawnio_open"), symbol("pawnio_close"))
    else {
        return PawnIoStatus::NotInstalled;
    };
    // SAFETY: the symbols come from PawnIOLib.dll and match PawnIOLib.h.
    let open: PawnioOpen = unsafe { std::mem::transmute(open_addr) };
    // SAFETY: as above.
    let close: PawnioClose = unsafe { std::mem::transmute(close_addr) };
    // SAFETY: as above.
    let version: Option<PawnioVersion> =
        symbol("pawnio_version").map(|addr| unsafe { std::mem::transmute(addr) });

    let mut handle: HANDLE = std::ptr::null_mut();
    // SAFETY: valid out-pointer; closed below when it succeeds.
    let hr = unsafe { open(&mut handle) };
    if hr < 0 || handle.is_null() {
        return if (hr as u32) == E_ACCESS_DENIED {
            PawnIoStatus::Denied
        } else {
            PawnIoStatus::Failed(describe(hr))
        };
    }

    let mut packed = 0u32;
    // SAFETY: `version` is a valid function pointer and `packed` a live u32.
    let version_hr = version.map(|f| unsafe { f(&mut packed) }).unwrap_or(-1);
    // SAFETY: `handle` came from `pawnio_open` and is closed once here.
    unsafe { close(handle) };

    let version = if version_hr >= 0 {
        format!(
            "{}.{}.{}",
            packed >> 16,
            (packed >> 8) & 0xFF,
            packed & 0xFF
        )
    } else {
        String::new()
    };
    PawnIoStatus::Ready { version }
}

/// PawnIO is Windows-only; elsewhere telemetry comes from sysfs/hwmon.
#[cfg(not(windows))]
pub fn status() -> PawnIoStatus {
    PawnIoStatus::Ready {
        version: String::new(),
    }
}

/// The bundled PawnIO installer, when the app ships one.
pub fn installer() -> Option<std::path::PathBuf> {
    let exe = std::env::current_exe().ok()?;
    let dir = exe.parent()?;
    ["installer/PawnIO_setup.exe", "PawnIO_setup.exe"]
        .iter()
        .map(|relative| dir.join(relative))
        .find(|path| path.is_file())
}

/// Runs the PawnIO installer with its own window (installing a kernel driver is
/// the user's decision, so no silent flags). The app is elevated, so no extra
/// UAC prompt is raised.
#[cfg(windows)]
pub fn run_installer(path: &std::path::Path) -> Result<(), String> {
    let status = std::process::Command::new(path)
        .status()
        .map_err(|error| format!("could not start the PawnIO installer: {error}"))?;
    if status.success() {
        Ok(())
    } else {
        Err(format!("the PawnIO installer exited with {status}"))
    }
}

#[cfg(not(windows))]
pub fn run_installer(_path: &std::path::Path) -> Result<(), String> {
    Err("PawnIO only applies to Windows".to_owned())
}
