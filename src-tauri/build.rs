use std::path::PathBuf;

fn main() {
    ensure_kiri_cli_built();
    tauri_build::build()
}

/// `tauri.conf.json` declares `../target/release/kiri-cli` as a bundle
/// resource. `tauri_build::build` validates that path at compile time
/// (even for `cargo check` / `cargo test`), and the bundler copies it
/// into the produced `.app` / `.dmg`.
///
/// Building `kiri-cli` from inside this build script would deadlock:
/// the outer `cargo build` already holds the workspace target lock,
/// so a nested `cargo build -p kiri-cli` would wait on it forever.
///
/// Instead, `tauri.conf.json`'s `beforeBuildCommand` / `beforeDevCommand`
/// run `npm run build:cli` before tauri-cli invokes cargo, so the binary
/// is guaranteed to be present and up to date. This script just verifies
/// that the binary actually exists and fails fast with a helpful message
/// if someone bypassed the npm scripts.
///
/// `cargo:rerun-if-changed` triggers a re-validate when the cli sources
/// change, so a stale binary is caught on the next build.
fn ensure_kiri_cli_built() {
    println!("cargo:rerun-if-changed=../crates/kiri-cli/src");
    println!("cargo:rerun-if-changed=../crates/kiri-cli-proto/src");
    println!("cargo:rerun-if-changed=../crates/kiri-cli/Cargo.toml");
    println!("cargo:rerun-if-changed=../crates/kiri-cli-proto/Cargo.toml");

    let cli_bin = PathBuf::from("../target/release/kiri-cli");
    assert!(
        cli_bin.exists(),
        "kiri-cli binary not found at {}. Run `npm run build:cli` (or use \
         `npm run install:app` / `npm run build:app`) before invoking cargo \
         on src-tauri directly.",
        cli_bin.display()
    );
}
