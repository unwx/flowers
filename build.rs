use std::env;

fn main() {
    let target_arch =
        env::var("CARGO_CFG_TARGET_ARCH").expect("failed to fetch build architecture");
    let allowed_arches = ["x86", "x86_64"];

    if !allowed_arches.contains(&target_arch.as_str()) {
        panic!(
            "This crate only supports the following architectures: '{}'. Current target architecture: '{}'",
            allowed_arches.join(", "),
            target_arch
        );
    }
}
