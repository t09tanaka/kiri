use std::process::Command;

fn main() {
    ensure_kiri_cli_built();
    tauri_build::build()
}

/// `tauri.conf.json` declares `../target/release/kiri-cli` as a bundle
/// resource. `tauri_build::build` validates resource paths at compile
/// time regardless of whether bundling is actually requested (e.g.
/// `cargo test` or `cargo check`), and the bundler later copies that
/// path into the produced `.app` / `.dmg`. So a fresh checkout fails
/// with `resource path ../target/release/kiri-cli doesn't exist`, and
/// an install rebuilt after edits to `kiri-cli`/`kiri-cli-proto`
/// would otherwise ship the previous binary because the file already
/// exists on disk.
///
/// Always shell out to `cargo build --release -p kiri-cli`. Cargo's
/// incremental build keeps the no-change case effectively free; when
/// sources do change it picks up the new binary before tauri-build's
/// resource check runs and before the bundler copies the file.
///
/// `cargo:rerun-if-changed` ensures the build script re-runs whenever
/// `kiri-cli` or its proto dependency changes, otherwise cargo would
/// cache the previous run's outputs and skip this step.
fn ensure_kiri_cli_built() {
    println!("cargo:rerun-if-changed=../crates/kiri-cli/src");
    println!("cargo:rerun-if-changed=../crates/kiri-cli-proto/src");
    println!("cargo:rerun-if-changed=../crates/kiri-cli/Cargo.toml");
    println!("cargo:rerun-if-changed=../crates/kiri-cli-proto/Cargo.toml");

    let cargo = std::env::var_os("CARGO").unwrap_or_else(|| "cargo".into());
    let status = Command::new(cargo)
        .args(["build", "--release", "-p", "kiri-cli"])
        .status()
        .expect("failed to invoke cargo for kiri-cli");
    assert!(status.success(), "kiri-cli build failed");
}
