fn main() {
    // Embed the application icon into basecreator.exe when building for Windows.
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("windows") {
        #[cfg(windows)]
        {
            let mut res = winresource::WindowsResource::new();
            res.set_icon("../../assets/basecreator.ico");
            let _ = res.compile();
        }
    }
    println!("cargo:rerun-if-changed=../../assets/basecreator.ico");
}
