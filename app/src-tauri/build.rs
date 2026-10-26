fn main() {
    // Windows: Tauri's default manifest plus the UTF-8 code page. SQLite 2
    // opens files through the ANSI API (CreateFileA), which otherwise reads
    // the UTF-8 paths it's given in the legacy code page and can't open
    // anything under a non-ASCII user name. Needs Windows 10 1903 or later.
    println!("cargo:rerun-if-changed=windows-app-manifest.xml");
    let windows = tauri_build::WindowsAttributes::new().app_manifest(include_str!("windows-app-manifest.xml"));
    tauri_build::try_build(tauri_build::Attributes::new().windows_attributes(windows)).expect("failed to run tauri-build");
}
