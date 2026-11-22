# sudocooler - Windows installer & portable package

These scripts build, stage and install a Windows release of **sudocooler** (the
NSH app) together with the **sudokooler** daemon from the git submodule. All
scripts target Windows 10/11 and PowerShell 5.1+; none of them need administrator
rights for the app itself.

## Files

| File | Purpose |
| --- | --- |
| `build-release.ps1` | Builds app + daemon, downloads PawnIO assets into build output, stages `dist\sudocooler-portable\` and zips it to `dist\sudocooler-portable.zip`. |
| `install.ps1` | Installs the extracted portable package per-user, creates a Start Menu shortcut, registers the elevated logon task, optionally installs PawnIO. |
| `uninstall.ps1` | Removes the task, shortcut, daemon path variable and install folder. |

## Prerequisites

- Rust toolchain (`cargo`) on PATH - normally `%USERPROFILE%\.cargo\bin`.
- NSH 0.5 CLI (`nsh`) on PATH - normally `%LOCALAPPDATA%\nsh\bin`. The scripts
  probe both of those locations automatically.
- The daemon submodule initialized: `git submodule update --init --recursive`.
- Internet access for the PawnIO downloads (or use `-SkipPawnIO` offline).

## 1. Build a portable release

From the repository root:

```powershell
powershell -ExecutionPolicy Bypass -File .\installer\build-release.ps1
```

Result:

```text
dist\sudocooler-portable\               staging folder (ready to run in place)
  sudocooler.exe                        NSH runner (app icon embedded)
  nsh\                                  NSH bundle + assets (icons, logo, ...)
  sudokooler.exe                        daemon, built from the submodule (release)
  pawnio\AMDFamily17.bin                signed AMD Zen 1-5 SMN/RAPL module
  pawnio\IntelMSR.bin                   signed Intel MSR module
  installer\PawnIO_setup.exe            official signed PawnIO driver installer
  install.ps1, uninstall.ps1
  README.md, APP_README.md, THIRD_PARTY_LICENSES.txt
dist\sudocooler-portable.zip            ready to share
```

Notes:

- NSH 0.5 has **no `--release` flag on `nsh package`**. The script runs
  `nsh build --release` first and then `nsh package --target windows`, which is
  the supported equivalent (the package command also builds its runner in
  release mode). The Dockerfile-free equivalent of "just build the exe" is
  **not** used: `cargo build --release` alone produces a bare exe without the
  NSH bundle/icons, which the packaged app needs.
- PawnIO binaries are downloaded from the official GitHub releases
  (`namazso/PawnIO.Setup`, `namazso/PawnIO.Modules`, the same sources the
  upstream daemon's `packaging\bundle-windows.ps1` uses). They are written only
  to `dist\` (gitignored) and a temporary folder that is deleted afterwards.
  **Proprietary/signed binaries are never committed.**
- Reuse existing binaries with `-AppPackageDir <dir>` (a directory containing
  `sudocooler.exe` + `nsh\`) and/or `-DaemonBinary <path>`.
- `-SkipPawnIO` builds without downloads; you must then install PawnIO manually
  for CPU temperature/power telemetry.

## 2. Use it portably

Extract the zip anywhere and run `sudocooler.exe`. The app finds the daemon via
the `SUDOKOOLER_BIN` environment variable (set by `install.ps1`) or a
`sudokooler.exe` next to it in the working directory. For portable use, launch
it from the extracted folder, or set the variable yourself:

```powershell
$env:SUDOKOOLER_BIN = "$PWD\sudokooler.exe"
.\sudocooler.exe
```

The daemon finds the signed module at `<exe_dir>\pawnio\AMDFamily17.bin`, via
`SUDOKOOLER_PAWNIO_MODULE`, or at
`C:\Program Files\PawnIO\modules\AMDFamily17.bin`.

## 3. Install per-user

From the **extracted** portable folder:

```powershell
powershell -ExecutionPolicy Bypass -File .\install.ps1
```

With PawnIO installation explicitly allowed:

```powershell
powershell -ExecutionPolicy Bypass -File .\install.ps1 -InstallPawnIO
```

The script:

1. Checks whether PawnIO is already installed. It installs it **only when
   missing and `-InstallPawnIO` is given**, using the bundled
   `installer\PawnIO_setup.exe` (downloading it to temp only if the bundle does
   not include it). The PawnIO installer may show a UAC prompt - the driver is
   privileged, even though sudocooler itself is installed without admin.
2. Stops only running copies inside the target folder, then copies app, daemon,
   `nsh\`, `pawnio\` and docs to
   `%LOCALAPPDATA%\Programs\sudocooler`. Re-running overwrites in place
   (**idempotent**).
3. Sets the per-user `SUDOKOOLER_BIN` variable to the installed daemon. This is
   necessary because the logon task starts the app with `System32` as its
   working directory, so the app's relative daemon lookup would otherwise fail.
4. Creates `%APPDATA%\Microsoft\Windows\Start Menu\Programs\sudocooler.lnk`.
5. Registers the exact task the app itself uses, so the in-app **Start at
   login** switch sees and can manage it:

   ```text
   schtasks /Create /TN sudocooler /TR "<install dir>\sudocooler.exe" /SC ONLOGON /RL HIGHEST /F
   ```

Every change is guarded by `ShouldProcess`, so `-WhatIf` previews everything
and `-Confirm` prompts. `-SkipTask` registers no task (the Start Menu shortcut
still works). A warning is printed (not an error) when PawnIO is missing and
`-InstallPawnIO` was not passed.

## 4. Uninstall

```powershell
powershell -ExecutionPolicy Bypass -File "$env:LOCALAPPDATA\Programs\sudocooler\uninstall.ps1"
```

or pass `-InstallDir <path>` to the script in this folder. It removes the
Scheduled Task, Start Menu shortcut, the `SUDOKOOLER_BIN` variable (only if it
points into the install folder), running installed processes and the folder.
`-WhatIf` / `-Confirm` are supported.

PawnIO is deliberately **left installed** - other tools (FanControl, OpenRGB,
etc.) may use the same driver. Remove it from Windows Settings -> Apps if you
are sure nothing else needs it.

## Elevation and telemetry caveats

- Reading CPU temperature and package power through PawnIO requires the signed
  kernel driver **and an elevated process**. That is why the logon task uses
  `/RL HIGHEST`, and why the app's **Start at login** setting creates a task
  rather than a Run-registry entry.
- Without PawnIO (or HWiNFO64 shared memory), the daemon still runs using Win32
  PDH, but reports `0.0 C` / `0 W`.
- The current daemon implementation only reads `AMDFamily17.bin` (AMD Zen).
  `IntelMSR.bin` is bundled for Intel systems/future support, but the daemon
  does not select it yet; set `SUDOKOOLER_PAWNIO_MODULE` explicitly if you use a
  modified daemon that supports it.
- A task created by your user with `/RL HIGHEST` runs with that user's elevated
  token at logon. On a standard (non-admin) account there is no elevation to
  grant, so PawnIO-backed telemetry will not work; use an account with admin
  rights and approve the PawnIO UAC prompt.
