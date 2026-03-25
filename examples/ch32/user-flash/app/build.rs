fn main() {
    let dir = std::env::var("CARGO_MANIFEST_DIR").unwrap();
    println!("cargo:rustc-link-search={dir}");
    println!("cargo:rerun-if-changed=memory.x");
    println!("cargo:rustc-link-arg=-Ttinyboot.x");
    println!("cargo:rustc-link-arg=-Tboot_request.x");
    println!("cargo:rustc-link-arg=-Tdefmt.x");
    println!("cargo:rustc-link-arg=--wrap=_setup_interrupts");
}
