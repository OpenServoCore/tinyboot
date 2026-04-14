fn main() {
    let out = std::path::PathBuf::from(std::env::var_os("OUT_DIR").unwrap());

    // Re-emit peripheral cfgs from tinyboot-ch32-hal (exposed via its `links` metadata).
    if let Ok(cfgs) = std::env::var("DEP_TINYBOOT_CH32_HAL_CFGS") {
        for cfg in cfgs.split(',').filter(|s| !s.is_empty()) {
            println!("cargo::rustc-check-cfg=cfg({cfg})");
            println!("cargo:rustc-cfg={cfg}");
        }
    }

    #[cfg(feature = "rt")]
    {
        std::fs::copy("link.x", out.join("link.x")).unwrap();
        println!("cargo:rerun-if-changed=link.x");
    }

    println!("cargo:rustc-link-search={}", out.display());
    println!("cargo:rerun-if-changed=build.rs");
}
