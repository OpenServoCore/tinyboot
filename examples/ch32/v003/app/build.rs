fn main() {
    let system_flash = cfg_has("CARGO_FEATURE_SYSTEM_FLASH");
    let user_flash = cfg_has("CARGO_FEATURE_USER_FLASH");

    match (system_flash, user_flash) {
        (true, false) | (false, true) => {}
        _ => panic!("Enable exactly one flash mode: `system-flash` or `user-flash`"),
    }

    let flash_mode = if system_flash {
        "system-flash"
    } else {
        "user-flash"
    };

    let manifest_dir = std::env::var("CARGO_MANIFEST_DIR").unwrap();
    let out_dir = std::env::var("OUT_DIR").unwrap();
    let src = format!("{manifest_dir}/memory_x/{flash_mode}.x");
    let dst = format!("{out_dir}/memory.x");
    std::fs::copy(&src, &dst).unwrap_or_else(|e| panic!("copy {src} -> {dst}: {e}"));

    println!("cargo:rustc-link-search={out_dir}");
    println!("cargo:rerun-if-changed=memory_x");
    println!("cargo:rustc-link-arg=-Ttb-app.x");
    println!("cargo:rustc-link-arg=-Tdefmt.x");

    if user_flash {
        println!("cargo:rustc-link-arg=-Ttb-user-flash.x");
        println!("cargo:rustc-link-arg=--wrap=_setup_interrupts");
    }
}

fn cfg_has(key: &str) -> bool {
    std::env::var_os(key).is_some()
}
