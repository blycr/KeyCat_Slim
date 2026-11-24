fn main() {
    println!("cargo:rerun-if-changed=../../assets/icons/icon.ico");
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() != Ok("windows") {
        return;
    }
    let mut res = winresource::WindowsResource::new();
    // Support cross-compiling from Linux/WSL using llvm-rc when rc.exe is not present.
    if std::env::var_os("CARGO_CFG_TARGET_OS").is_some() && !cfg!(windows) {
        res.set_toolkit_path("/usr/bin");
        res.set_windres_path("llvm-rc");
        res.set_ar_path("llvm-lib");
    }
    res.set_icon("../../assets/icons/icon.ico");
    res.set_manifest(
        r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<assembly xmlns="urn:schemas-microsoft-com:asm.v1" manifestVersion="1.0">
  <application xmlns="urn:schemas-microsoft-com:asm.v3">
    <windowsSettings>
      <dpiAwareness xmlns="http://schemas.microsoft.com/SMI/2016/WindowsSettings">PerMonitorV2</dpiAwareness>
    </windowsSettings>
  </application>
</assembly>"#,
    );
    if let Err(e) = res.compile() {
        println!("cargo:warning=winresource compile failed: {e}");
    }
}
