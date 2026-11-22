#!/usr/bin/env bash
# Builds and stages a portable Linux release of sudocooler.
#
# 1. Builds the optimized NSH app (nsh build --release) and packages the Linux
#    runner (nsh package --target linux).
# 2. Builds the sudokooler daemon from the git submodule in release mode.
# 3. Stages dist/sudocooler-linux-portable/ with the app dir, daemon, docs and
#    the upstream udev rule, then creates dist/sudocooler-linux-portable.tar.gz.
# 4. Copies the NSH package archive to
#    dist/sudocooler-<version>-linux-x86_64.tar.gz so `nsh update-manifest`
#    can publish it alongside the Windows archive in latest.json.
#
# No administrator rights are required. Nothing is downloaded and no generated
# output is written outside dist/ (which is gitignored).

set -euo pipefail

# Make the documented NSH / Cargo install locations available to this process.
for dir in "$HOME/.cargo/bin" "$HOME/.local/bin"; do
    case ":$PATH:" in
        *":$dir:"*) ;;
        *) PATH="$dir:$PATH" ;;
    esac
done
export PATH

die() { printf 'error: %s\n' "$*" >&2; exit 1; }
step() { printf '\n==> %s\n' "$*"; }
detail() { printf '    %s\n' "$*"; }

require_cmd() {
    command -v "$1" >/dev/null 2>&1 || die "'$1' was not found on PATH. Install it and retry."
}

require_cmd nsh
require_cmd cargo
require_cmd tar

script_dir="$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)"
repo_root="$(CDPATH= cd -- "$script_dir/.." && pwd)"

out_dir="${OUT_DIR:-$repo_root/dist}"
stage_name='sudocooler-linux-portable'
stage_dir="$out_dir/$stage_name"
portable_archive="$out_dir/$stage_name.tar.gz"
nsh_json="$repo_root/nsh.json"

[ -f "$nsh_json" ] || die "nsh.json not found at '$nsh_json'."

# python3 is present on the Linux CI runners; sed is a fallback for minimal systems.
read_version() {
    if command -v python3 >/dev/null 2>&1; then
        python3 -c 'import json,sys; print(json.load(open(sys.argv[1], encoding="utf-8"))["version"])' "$nsh_json"
    else
        sed -n 's/.*"version"[[:space:]]*:[[:space:]]*"\([^"]*\)".*/\1/p' "$nsh_json" | head -n 1
    fi
}

version="$(read_version)"
[ -n "$version" ] || die "Could not read 'version' from '$nsh_json'."
update_archive="$out_dir/sudocooler-${version}-linux-x86_64.tar.gz"

daemon_manifest="$repo_root/sudokooler/Cargo.toml"
[ -f "$daemon_manifest" ] || die "Daemon submodule missing at '$daemon_manifest'. Run: git submodule update --init --recursive"

tmp_dir="$(mktemp -d)"
trap 'rm -rf "$tmp_dir"' EXIT
package_out="$tmp_dir/nsh-package"

step 'Preparing output directories'
mkdir -p "$out_dir"
rm -rf "$stage_dir"
rm -f "$portable_archive" "$update_archive"
detail "Repository : $repo_root"
detail "Output     : $out_dir"
detail "Version    : $version"

step '[1/4] Building optimized NSH app (nsh build --release) ...'
(cd "$repo_root" && nsh build --release)

step '[2/4] Packaging the Linux runner (nsh package --target linux) ...'
mkdir -p "$package_out"
(cd "$repo_root" && nsh package --target linux --out "$package_out")

app_dir="$package_out/sudocooler"
[ -d "$app_dir" ] || die "NSH package did not produce '$app_dir'."

package_archive="$package_out/sudocooler.tar.gz"
if [ ! -f "$package_archive" ]; then
    # Fallback: some NSH versions write the archive into the default output dir.
    package_archive="$repo_root/.nsh/out/sudocooler.tar.gz"
fi
[ -f "$package_archive" ] || die "NSH package did not produce 'sudocooler.tar.gz'."

step '[3/4] Building the daemon (sudokooler submodule, release) ...'
(cd "$repo_root" && cargo build --release --manifest-path "$daemon_manifest")
daemon_bin="$repo_root/sudokooler/target/release/sudokooler"
[ -f "$daemon_bin" ] || die "Daemon binary not found at '$daemon_bin'."
[ -x "$daemon_bin" ] || chmod +x "$daemon_bin"

# The self-update archive is the NSH app package, which does not include the
# daemon. Add `sudokooler` beside the runner inside it so an updated install
# can still read telemetry (cli.rs looks for the daemon next to the app).
cp -f "$daemon_bin" "$app_dir/sudokooler"
chmod +x "$app_dir/sudokooler"
tar -czf "$update_archive" -C "$package_out" sudocooler
detail "Update archive (with daemon): $update_archive"

step '[4/4] Staging the portable folder and creating the archive ...'
mkdir -p "$stage_dir"
cp -R "$app_dir" "$stage_dir/sudocooler"
cp -f "$daemon_bin" "$stage_dir/sudokooler"
chmod +x "$stage_dir/sudokooler"

cp -f "$script_dir/README.md" "$stage_dir/INSTALLER_README.md"
cp -f "$repo_root/README.md" "$stage_dir/APP_README.md"

cat > "$stage_dir/README.md" <<'EOF'
# sudocooler - Linux portable package

## Run

From this folder, point the app at the bundled daemon and start the runner
inside the `sudocooler/` NSH bundle (the executable is normally
`sudocooler/sudocooler`):

    SUDOKOOLER_BIN="$PWD/sudokooler" ./sudocooler/sudocooler

The app also looks for a `sudokooler` binary next to its own executable and in
`nsh/`, so setting `SUDOKOOLER_BIN` is the reliable option for a portable
extract.

## HID access (required)

The daemon talks to the cooler over `/dev/hidraw*`. Unprivileged users need the
included upstream udev rule:

    sudo cp 99-sudokooler.rules /etc/udev/rules.d/
    sudo udevadm control --reload-rules
    sudo udevadm trigger

Replug the cooler or reboot afterwards. The rule matches Sudokoo HID devices
(`SUBSYSTEM=="hidraw"`, `ATTRS{idVendor}=="381c"`) with `MODE="0660"` and the
`uaccess` tag. Without it the app can still list devices, but reads/writes fail
with permission denied.

## Contents

    sudocooler/                  NSH app bundle (runner + assets)
    sudokooler                   daemon (release build from the submodule)
    99-sudokooler.rules          upstream udev rule for HID access
    README.md                    this file
    INSTALLER_README.md          build/install documentation
    APP_README.md                upstream app documentation
    THIRD_PARTY_LICENSES.txt     license summary

No PawnIO or other signed/proprietary binaries are downloaded or shipped on
Linux: the daemon reads sensors from sysfs and /proc directly.
EOF

cat > "$stage_dir/THIRD_PARTY_LICENSES.txt" <<'EOF'
sudocooler Linux Portable Release - Third-Party Licenses and Sources
====================================================================

1. sudocooler app (sudocooler/ NSH bundle) - GPL-3.0
   Source: https://github.com/msartore/sudocooler

2. sudokooler daemon (sudokooler) - MIT
   Source: https://github.com/BGW-10/sudokooler

No signed or proprietary binaries are bundled in the Linux release. The daemon
uses the system hidraw/libusb stack; the included 99-sudokooler.rules is the
upstream udev rule from the daemon project.
EOF

rules_src="$repo_root/sudokooler/packaging/99-sudokooler.rules"
if [ -f "$rules_src" ]; then
    cp -f "$rules_src" "$stage_dir/99-sudokooler.rules"
    detail 'udev rule: 99-sudokooler.rules (copy to /etc/udev/rules.d/)'
else
    printf 'warning: udev rule not found at %s\n' "$rules_src" >&2
fi

tar -czf "$portable_archive" -C "$out_dir" "$stage_name"

printf '\nBuild complete.\n'
printf '  Portable folder : %s\n' "$stage_dir"
printf '  Portable archive: %s\n' "$portable_archive"
printf '  Update archive  : %s\n' "$update_archive"
