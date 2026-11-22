//! Embeds the app icon into the Windows executable (title bar, taskbar,
//! Alt+Tab and Explorer). `nsh.json` carries the icon for packaged builds.

fn main() {
    #[cfg(target_os = "windows")]
    {
        let mut resource = winres::WindowsResource::new();
        resource.set_icon("assets/icon.ico");
        if let Err(error) = resource.compile() {
            println!("cargo:warning=app icon could not be embedded: {error}");
        }
    }
}
