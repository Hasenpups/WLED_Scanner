fn main() {
    println!("cargo:rerun-if-changed=assets/icon.ico");
    // Embed the icon into the .exe so Explorer and the taskbar show it.
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("windows") {
        winresource::WindowsResource::new()
            .set_icon("assets/icon.ico")
            .compile()
            .expect("failed to embed Windows icon resource");
    }
}
