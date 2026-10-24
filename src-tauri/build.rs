fn main() {
    // windres (GNU toolchain) fails on non-ASCII paths, so hand it an icon copy
    // from the ASCII target directory instead of the project folder.
    let manifest_dir = std::path::PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").unwrap());
    let out_dir = std::path::PathBuf::from(std::env::var("OUT_DIR").unwrap());
    let icon_src = manifest_dir.join("icons").join("icon.ico");
    let icon_copy = out_dir.join("udm-icon.ico");
    if let Err(e) = std::fs::copy(&icon_src, &icon_copy) {
        panic!("failed to copy {} for windres: {e}", icon_src.display());
    }

    let attrs = tauri_build::Attributes::new()
        .windows_attributes(tauri_build::WindowsAttributes::new().window_icon_path(&icon_copy));
    tauri_build::try_build(attrs).expect("failed to run tauri-build");
}
