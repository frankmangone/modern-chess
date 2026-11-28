//! Builds the engine for WebAssembly (`modern-chess-sandbox-guest`) so the host can embed it.
//! Needs the `wasm32-unknown-unknown` target: `rustup target add wasm32-unknown-unknown`.

use std::env;
use std::path::PathBuf;
use std::process::Command;

#[allow(clippy::unwrap_used, clippy::expect_used)]
fn main() {
    let root = PathBuf::from(env::var("CARGO_MANIFEST_DIR").expect("manifest dir")).join("../..");
    let out = PathBuf::from(env::var("OUT_DIR").expect("out dir"));
    let target_dir = out.join("guest-target");

    for watched in ["engine", "session", "sandbox-protocol", "sandbox-guest"] {
        println!(
            "cargo:rerun-if-changed={}",
            root.join("crates").join(watched).join("src").display()
        );
        println!(
            "cargo:rerun-if-changed={}",
            root.join("crates")
                .join(watched)
                .join("Cargo.toml")
                .display()
        );
    }
    println!("cargo:rerun-if-changed=build.rs");

    let cargo = env::var("CARGO").unwrap_or_else(|_| "cargo".to_string());
    let mut build = Command::new(cargo);
    build
        .current_dir(&root)
        .args([
            "build",
            "--release",
            "--target",
            "wasm32-unknown-unknown",
            "-p",
            "modern-chess-sandbox-guest",
            "--target-dir",
        ])
        .arg(&target_dir);
    // The outer build's flags are for the host; the module is built by the plain rules.
    for variable in [
        "RUSTFLAGS",
        "CARGO_ENCODED_RUSTFLAGS",
        "RUSTC_WRAPPER",
        "RUSTC_WORKSPACE_WRAPPER",
    ] {
        build.env_remove(variable);
    }
    let status = build
        .status()
        .expect("could not run cargo for the sandbox module");
    assert!(
        status.success(),
        "building the sandbox module failed: is the wasm32-unknown-unknown target installed? \
         (rustup target add wasm32-unknown-unknown)"
    );

    let module = target_dir.join("wasm32-unknown-unknown/release/modern_chess_sandbox_guest.wasm");
    std::fs::copy(&module, out.join("guest.wasm")).expect("copying the sandbox module");
}
