//! Embeds the files the game seeds on startup: the engine's specs and this crate's pages.
//! Both directories are scanned here, so adding a file needs no code change.

#![allow(clippy::unwrap_used, clippy::expect_used)] // a build script should stop the build

use std::fs;
use std::path::{Path, PathBuf};

fn json_files(dir: &Path) -> Vec<PathBuf> {
    let mut files: Vec<PathBuf> = fs::read_dir(dir)
        .unwrap_or_else(|err| panic!("cannot read {}: {err}", dir.display()))
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|path| path.extension().is_some_and(|ext| ext == "json"))
        .collect();
    files.sort();
    files
}

fn main() {
    println!("cargo:rerun-if-changed=migrations");

    let manifest = PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").unwrap());
    let mut code = String::new();

    for (constant, dir) in [
        ("SPEC_FILES", manifest.join("../engine/specs")),
        ("PAGE_FILES", manifest.join("pages")),
    ] {
        println!("cargo:rerun-if-changed={}", dir.display());
        code.push_str(&format!(
            "pub(crate) const {constant}: &[(&str, &str)] = &[\n"
        ));
        for file in json_files(&dir) {
            println!("cargo:rerun-if-changed={}", file.display());
            let stem = file.file_stem().unwrap().to_string_lossy().into_owned();
            let absolute = fs::canonicalize(&file).unwrap();
            code.push_str(&format!(
                "    ({stem:?}, include_str!({:?})),\n",
                absolute.to_string_lossy()
            ));
        }
        code.push_str("];\n");
    }

    let out = PathBuf::from(std::env::var("OUT_DIR").unwrap()).join("seed_files.rs");
    fs::write(out, code).unwrap();
}
