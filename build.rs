use std::{env, process::Command};

fn main() {
    build_native_platform();
    println!("cargo:rerun-if-env-changed=GIT_COMMIT_HASH");
    println!("cargo:rerun-if-changed=.git/HEAD");
    println!("cargo:rerun-if-changed=.git/refs/heads");

    let commit_hash = env::var("GIT_COMMIT_HASH")
        .ok()
        .and_then(normalize_commit_hash)
        .or_else(git_commit_hash)
        .unwrap_or_else(|| "unknown".into());
    println!("cargo:rustc-env=GIT_COMMIT_HASH={commit_hash}");
}

fn build_native_platform() {
    if env::var("CARGO_CFG_TARGET_OS").as_deref() != Ok("macos") {
        return;
    }
    let output = std::path::PathBuf::from(env::var_os("OUT_DIR").expect("Cargo OUT_DIR"));
    let arch = match env::var("CARGO_CFG_TARGET_ARCH").as_deref() {
        Ok("aarch64") => "arm64",
        Ok("x86_64") => "x86_64",
        _ => panic!("Unsupported macOS architecture"),
    };
    println!("cargo:rerun-if-changed=native/Platform.h");
    let source = "Hotkey";
    let path = format!("native/{source}.m");
    println!("cargo:rerun-if-changed={path}");
    let object = output.join(format!("{source}.o"));
    let status = Command::new("/usr/bin/xcrun")
        .args([
            "clang",
            "-c",
            "-fobjc-arc",
            "-fblocks",
            "-O2",
            "-Wall",
            "-Wextra",
            "-Werror",
            "-Wno-unused-parameter",
            "-Wno-deprecated-declarations",
            "-mmacosx-version-min=13.0",
            "-arch",
            arch,
        ])
        .arg(&path)
        .arg("-o")
        .arg(&object)
        .status()
        .expect("Apple clang is required");
    assert!(status.success(), "Could not compile {path}");
    let status = Command::new("/usr/bin/xcrun")
        .args(["libtool", "-static", "-o"])
        .arg(output.join("libgatto_platform.a"))
        .arg(object)
        .status()
        .expect("Apple libtool is required");
    assert!(
        status.success(),
        "Could not archive the native platform adapter"
    );
    println!("cargo:rustc-link-search=native={}", output.display());
    println!("cargo:rustc-link-lib=static=gatto_platform");
    println!("cargo:rustc-link-lib=framework=Foundation");
    println!("cargo:rustc-link-lib=framework=AppKit");
    println!("cargo:rustc-link-lib=framework=Carbon");
    println!("cargo:rustc-link-lib=framework=CoreGraphics");
}

fn git_commit_hash() -> Option<String> {
    let output = Command::new("git")
        .args(["rev-parse", "--short=12", "HEAD"])
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    normalize_commit_hash(String::from_utf8(output.stdout).ok()?)
}

fn normalize_commit_hash(value: String) -> Option<String> {
    let hash = value.trim();
    (hash.len() >= 7 && hash.len() <= 64 && hash.bytes().all(|byte| byte.is_ascii_hexdigit()))
        .then(|| hash.to_ascii_lowercase())
}
