fn main() {
    slint_build::compile("ui/main.slint").expect("the .slint files compile");
    windows_resources();
}

/// Gives the .exe file its icon, so that Explorer and the taskbar show it.
/// The build script runs on the computer that builds, so this happens when
/// Windows builds for Windows.
#[cfg(windows)]
fn windows_resources() {
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() != Ok("windows") {
        return;
    }
    let mut resource = winresource::WindowsResource::new();
    resource
        .set_icon("../../assets/icons/hotdogstand.ico")
        .set("ProductName", "HotDogStand")
        .set("FileDescription", "HotDogStand");
    resource
        .compile()
        .expect("the Windows resources compile");
}

#[cfg(not(windows))]
fn windows_resources() {}
