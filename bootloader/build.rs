use std::env;
use std::fs::File;
use std::io::Write;
use std::path::PathBuf;

fn main() {
    let out = &PathBuf::from(env::var_os("OUT_DIR").unwrap());
    File::create(out.join("memory.x"))
        .unwrap()
        .write_all(include_bytes!("memory.x"))
        .unwrap();

    if env::var_os("CARGO_FEATURE_LOG").is_none() {
        // stub defmt.x to prevent linker errors when defmt is not used
        File::create(out.join("defmt.x")).unwrap();
    }
    println!("cargo:rustc-link-search={}", out.display());
    println!("cargo:rerun-if-changed=memory.x");
}
