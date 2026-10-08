use std::{env, process::Command};

fn main() {
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
