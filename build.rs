//! On Windows, embeds the icon (`docs/icons/xylem.ico`) and the version
//! information in `xy.exe`, for Explorer and the taskbar.

fn main() {
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-changed=docs/icons/xylem.ico");
    // Building on Windows for Windows, as the CI and the releases do.
    #[cfg(windows)]
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("windows") {
        let mut res = winresource::WindowsResource::new();
        res.set_icon("docs/icons/xylem.ico");
        res.compile().expect("cannot embed the icon in xy.exe");
    }
}
