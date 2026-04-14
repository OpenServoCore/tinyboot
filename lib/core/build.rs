fn main() {
    let out = std::path::PathBuf::from(std::env::var_os("OUT_DIR").unwrap());

    for script in ["tb-boot.x", "tb-app.x"] {
        std::fs::copy(script, out.join(script)).unwrap();
        println!("cargo:rerun-if-changed={script}");
    }

    println!("cargo:rustc-link-search={}", out.display());
    println!("cargo:rerun-if-changed=build.rs");
}
