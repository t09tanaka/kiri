use std::path::PathBuf;
use std::process::Command;

fn main() {
    ensure_kiri_cli_built();
    tauri_build::build()
}

/// `tauri.conf.json` declares `../target/release/kiri-cli` as a bundle
/// resource. `tauri_build::build` validates resource paths at compile time
/// regardless of whether bundling is actually requested (e.g. `cargo test`
/// or `cargo check`), so a fresh checkout that has not run a full
/// workspace `cargo build --release` will fail with
/// `resource path ../target/release/kiri-cli doesn't exist`.
///
/// Build the binary from build.rs so the workspace is self-contained.
/// `cargo` handles incremental builds, so this is a no-op after the
/// first invocation until `kiri-cli` sources change.
fn ensure_kiri_cli_built() {
    let cli_bin = PathBuf::from("../target/release/kiri-cli");
    if !cli_bin.exists() {
        let cargo = std::env::var_os("CARGO").unwrap_or_else(|| "cargo".into());
        let status = Command::new(cargo)
            .args(["build", "--release", "-p", "kiri-cli"])
            .status()
            .expect("failed to invoke cargo for kiri-cli");
        assert!(status.success(), "kiri-cli build failed");
    }
    println!("cargo:rerun-if-changed=../crates/kiri-cli/src");
    println!("cargo:rerun-if-changed=../crates/kiri-cli-proto/src");
    println!("cargo:rerun-if-changed=../crates/kiri-cli/Cargo.toml");
    println!("cargo:rerun-if-changed=../crates/kiri-cli-proto/Cargo.toml");
}
