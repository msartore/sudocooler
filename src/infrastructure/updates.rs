//! Self-update for packaged builds, using NSH's verified updater.
//!
//! The manifest comes from GitHub Releases. NSH verifies its app id, version
//! ordering, HTTPS URL and archive hash before we hand the archive to its
//! installer. Development builds report that updates need a packaged app.

use std::io::Read;

use nsh_core::element::desktop::restart;
use nsh_updater::install::Installation;
use nsh_updater::{Config, Download, platform};

const APP_ID: &str = "io.sudocooler.app";
const REPOSITORY: &str = "msartore/sudocooler";

fn config() -> Result<Config, String> {
    let mut config = Config::default();
    config.id = APP_ID.to_owned();
    config.version = env!("CARGO_PKG_VERSION").to_owned();
    config.github = Some(REPOSITORY.to_owned());
    config.validate().map_err(|error| error.to_string())?;
    Ok(config)
}

fn http_get(url: &str) -> Result<Vec<u8>, String> {
    let response = ureq::get(url).call().map_err(|error| error.to_string())?;
    let mut reader = response.into_reader();
    let mut bytes = Vec::new();
    reader
        .read_to_end(&mut bytes)
        .map_err(|error| error.to_string())?;
    Ok(bytes)
}

/// Checks GitHub Releases and returns a human-readable result.
pub fn check() -> Result<String, String> {
    let config = config()?;
    let manifest = http_get(&config.manifest_url())?;
    match config
        .check(&manifest, None, &platform())
        .map_err(|error| error.to_string())?
    {
        Some(available) => Ok(format!(
            "v{} available{}",
            available.version,
            available
                .notes
                .map(|notes| format!(" — {notes}"))
                .unwrap_or_default()
        )),
        None => Ok(format!("v{} is up to date", config.version)),
    }
}

/// Downloads, verifies and installs the newest release, then restarts.
pub fn install() -> Result<String, String> {
    let config = config()?;
    let manifest = http_get(&config.manifest_url())?;
    let Some(available) = config
        .check(&manifest, None, &platform())
        .map_err(|error| error.to_string())?
    else {
        return Ok(format!("v{} is up to date", config.version));
    };
    let sha256 = available
        .sha256
        .ok_or_else(|| "the update has no archive checksum".to_owned())?;
    let archive = http_get(&available.url)?;
    let mut download = Download::new(&sha256, available.size);
    download
        .update(&archive)
        .map_err(|error| error.to_string())?;
    download.finish().map_err(|error| error.to_string())?;

    let path = std::env::temp_dir().join("sudocooler-update.zip");
    std::fs::write(&path, &archive).map_err(|error| error.to_string())?;
    let executable = std::env::current_exe().map_err(|error| error.to_string())?;
    let root = executable
        .parent()
        .ok_or_else(|| "cannot find the app directory".to_owned())?;
    let installation = Installation::directory(root, &executable)
        .ok_or_else(|| "this build cannot be updated in place".to_owned())?;
    if !installation.can_install() {
        return Err(
            "the app is installed where it cannot be replaced; use the setup installer".to_owned(),
        );
    }
    installation
        .install(&path)
        .map_err(|error| error.to_string())?;
    restart();
    Ok(format!("installed v{}", available.version))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_a_newer_manifest() {
        let config = config().unwrap();
        let sha256 = "0".repeat(64);
        let manifest = format!(
            r#"{{"id":"{APP_ID}","version":"99.0.0","platforms":{{"{}":{{"url":"https://example.com/app.zip","sha256":"{sha256}","size":1}}}}}}"#,
            platform()
        );
        let available = config
            .check(manifest.as_bytes(), None, &platform())
            .unwrap()
            .unwrap();
        assert_eq!(available.version, "99.0.0");
    }
}
