fn main() {
    println!("cargo:rerun-if-changed=assets/icon.ico");
    println!("cargo:rerun-if-changed=assets/icon.png");
    println!("cargo:rustc-check-cfg=cfg(has_app_icon)");
    if std::path::Path::new("assets/icon.png").is_file() {
        println!("cargo:rustc-cfg=has_app_icon");
    }
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("windows") {
        let mut resource = winresource::WindowsResource::new();
        if std::path::Path::new("assets/icon.ico").is_file() {
            resource.set_icon("assets/icon.ico");
        }
        resource
            .set("ProductName", "寶藏小助手")
            .set("FileDescription", "寶藏小助手")
            .compile()
            .expect("failed to compile Windows resources");
    }
}
