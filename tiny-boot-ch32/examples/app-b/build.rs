fn main() {
    if cfg!(feature = "defmt") {
        println!("cargo:rustc-link-arg=-Tdefmt.x");
    }
}
