fn main() {
    println!("cargo:rerun-if-changed=assets/icon/app.ico");
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("windows") {
        winresource::WindowsResource::new()
            .set_icon("assets/icon/app.ico")
            .set("ProductName", "Mathematical playground")
            .set("FileDescription", "Mathematical playground")
            .compile()
            .expect("compile Windows application icon");
    }
}
