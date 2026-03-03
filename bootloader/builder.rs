use std::{env, fs, path::PathBuf};

fn main() {
    let memory_x = if cfg!(feature = "ch32v003f4u6") {
        "memory/ch32v003f4u6.x"
    } else {
        panic!("Select exactly one chip feature (e.g. --features ch32v003f4u6)");
    };

    let manifest = PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap());
    let src = manifest.join(memory_x);

    let out = PathBuf::from(env::var("OUT_DIR").unwrap());
    let dst = out.join("memory.x");

    fs::copy(&src, &dst).expect("copy memory.x");

    // Ensure the linker can find memory.x
    println!("cargo:rustc-link-search={}", out.display());
    println!("cargo:rerun-if-changed={}", src.display());
}
