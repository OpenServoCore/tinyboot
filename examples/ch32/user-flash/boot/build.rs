fn main() {
    let dir = std::env::var("CARGO_MANIFEST_DIR").unwrap();
    println!("cargo:rustc-link-search={dir}");
    println!("cargo:rerun-if-changed=memory.x");
    println!("cargo:rustc-link-arg=-Ttb-boot.x");
    println!("cargo:rustc-link-arg=-Ttb-user-flash.x");
    println!("cargo:rustc-link-arg=-Tdefmt.x");
}
