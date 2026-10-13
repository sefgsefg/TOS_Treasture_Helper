fn main() {
    println!("cargo:rerun-if-changed=assets/icon.ico");
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("windows") {
        winresource::WindowsResource::new()
            .set_icon("assets/icon.ico")
            .set("ProductName", "寶藏小助手")
            .set("FileDescription", "寶藏小助手")
            .compile()
            .expect("failed to compile Windows icon resources");
    }
}
