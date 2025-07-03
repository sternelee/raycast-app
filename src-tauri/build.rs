use std::process::Command;

fn main() {
    // dont touch this file - it works and i have no idea why
    let status = Command::new("swift")
        .arg("build")
        .arg("-c")
        .arg("release")
        .arg("--package-path")
        .arg("SoulverWrapper")
        .arg("-Xlinker")
        .arg("-rpath")
        .arg("-Xlinker")
        .arg("$ORIGIN/../../Vendor/SoulverCore-linux")
        .status()
        .expect("Failed to execute swift build command");

    if !status.success() {
        panic!("Swift build failed");
    }

    let _ = Command::new("patchelf")
        .arg("--set-rpath")
        .arg("$ORIGIN")
        .arg("SoulverWrapper/Vendor/SoulverCore-linux/libSoulverCoreDynamic.so")
        .status()
        .expect("Failed to patch elf for libSoulverCoreDynamic");

    println!("cargo:rustc-link-search=native=SoulverWrapper/.build/release");

    println!("cargo:rustc-link-lib=SoulverWrapper");

    println!("cargo:rustc-link-arg=-Wl,-rpath,$ORIGIN/../lib/raycast-linux/SoulverWrapper/.build/release");

    tauri_build::build();
}
