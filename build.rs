use std::env;
use std::path::PathBuf;
use std::process::Command;

fn run(command: &mut Command) {
    let status = command.status().expect("failed to launch build command");
    assert!(status.success(), "build command failed: {command:?}");
}

fn main() {
    println!("cargo:rerun-if-changed=src/tray.m");

    let output = PathBuf::from(env::var("OUT_DIR").unwrap());
    let object = output.join("tray.o");
    let library = output.join("libtray_bridge.a");

    let sdk = Command::new("xcrun")
        .args(["--sdk", "macosx", "--show-sdk-path"])
        .output()
        .expect("failed to locate the macOS SDK");
    assert!(sdk.status.success(), "xcrun could not locate the macOS SDK");
    let sdk_path = String::from_utf8(sdk.stdout).unwrap();
    let target_arch = env::var("CARGO_CFG_TARGET_ARCH").unwrap();
    let clang_arch = match target_arch.as_str() {
        "x86_64" => "x86_64",
        "aarch64" => "arm64",
        other => panic!("unsupported macOS architecture: {other}"),
    };

    run(Command::new("clang")
        .args([
            "-fobjc-arc",
            "-mmacosx-version-min=11.0",
            "-arch",
            clang_arch,
            "-isysroot",
            sdk_path.trim(),
            "-c",
            "src/tray.m",
            "-o",
        ])
        .arg(&object));

    let _ = std::fs::remove_file(&library);
    run(Command::new("ar").arg("crs").arg(&library).arg(&object));

    println!("cargo:rustc-link-search=native={}", output.display());
    println!("cargo:rustc-link-lib=static=tray_bridge");
    println!("cargo:rustc-link-lib=framework=AppKit");
    println!("cargo:rustc-link-lib=framework=Foundation");
}
