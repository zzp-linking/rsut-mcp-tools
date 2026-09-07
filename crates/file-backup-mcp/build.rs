// build.rs
fn main() {
    // 只在 Windows 平台上执行图标嵌入
    if std::env::var("CARGO_CFG_TARGET_OS").unwrap() == "windows" {
        let mut res = winres::WindowsResource::new();
        res.set_icon("bear.ico"); // 刚才准备的图标路径
        res.compile().unwrap();
    }
}