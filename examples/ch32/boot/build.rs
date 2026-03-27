fn main() {
    let system_flash = cfg_has("CARGO_FEATURE_SYSTEM_FLASH");
    let user_flash = cfg_has("CARGO_FEATURE_USER_FLASH");

    match (system_flash, user_flash) {
        (true, false) | (false, true) => {}
        _ => panic!("Enable exactly one flash mode: `system-flash` or `user-flash`"),
    }

    let chips = [
        "CARGO_FEATURE_CH32V003F4P6",
        "CARGO_FEATURE_CH32V003A4M6",
        "CARGO_FEATURE_CH32V003F4U6",
        "CARGO_FEATURE_CH32V003J4M6",
    ];
    let selected: Vec<&str> = chips.iter().filter(|c| cfg_has(c)).copied().collect();
    if selected.len() != 1 {
        panic!("Enable exactly one chip feature");
    }

    let flash_mode = if system_flash {
        "system-flash"
    } else {
        "user-flash"
    };
    let chip = selected[0]
        .strip_prefix("CARGO_FEATURE_")
        .unwrap()
        .to_lowercase();

    let manifest_dir = std::env::var("CARGO_MANIFEST_DIR").unwrap();
    let out_dir = std::env::var("OUT_DIR").unwrap();
    let src = format!("{manifest_dir}/memory_x/{flash_mode}/{chip}.x");
    let dst = format!("{out_dir}/memory.x");
    std::fs::copy(&src, &dst).unwrap_or_else(|e| panic!("copy {src} -> {dst}: {e}"));

    println!("cargo:rustc-link-search={out_dir}");
    println!("cargo:rerun-if-changed=memory_x");
    println!("cargo:rustc-link-arg=-Ttb-boot.x");

    if user_flash {
        println!("cargo:rustc-link-arg=-Ttb-user-flash.x");
        println!("cargo:rustc-link-arg=-Tdefmt.x");
    }
}

fn cfg_has(key: &str) -> bool {
    std::env::var_os(key).is_some()
}
