use log::info;

fn main() {
    println!("cargo:rustc-env=TARGET={}", std::env::var("TARGET").unwrap());
    info!("Starting build");
    std::process::Command::new("cargo")
        .arg("xtask")
        .arg("build")
        .status()
        .unwrap();

    info!("Building tauri");
    tauri_build::build()
}
