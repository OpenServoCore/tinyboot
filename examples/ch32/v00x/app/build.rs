const CHIPS: &[&str] = &["ch32v006x8x6"];

fn main() {
    let system_flash = cfg_has("CARGO_FEATURE_SYSTEM_FLASH");
    let user_flash = cfg_has("CARGO_FEATURE_USER_FLASH");
    let standalone = cfg_has("CARGO_FEATURE_STANDALONE");

    let flash_mode = if standalone {
        "standalone"
    } else {
        match (system_flash, user_flash) {
            (true, false) => "system-flash",
            (false, true) => "user-flash",
            _ => panic!(
                "Enable exactly one flash mode: `system-flash`, `user-flash`, or `standalone`"
            ),
        }
    };

    let chip = CHIPS
        .iter()
        .find(|c| cfg_has(&format!("CARGO_FEATURE_{}", c.to_uppercase())))
        .expect("No chip variant selected");

    let manifest_dir = std::env::var("CARGO_MANIFEST_DIR").unwrap();
    let out_dir = std::env::var("OUT_DIR").unwrap();
    let src = format!("{manifest_dir}/memory_x/{chip}/{flash_mode}.x");
    let dst = format!("{out_dir}/memory.x");
    std::fs::copy(&src, &dst).unwrap_or_else(|e| panic!("copy {src} -> {dst}: {e}"));

    println!("cargo:rustc-link-search={out_dir}");
    println!("cargo:rerun-if-changed=memory_x");
    if standalone {
        println!("cargo:rustc-link-arg=-Tlink.x");
    } else {
        println!("cargo:rustc-link-arg=-Ttb-app.x");
        println!("cargo:rustc-link-arg=-Ttb-run-mode.x");
    }
    println!("cargo:rustc-link-arg=-Tdefmt.x");
}

fn cfg_has(key: &str) -> bool {
    std::env::var_os(key).is_some()
}
