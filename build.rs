use std::{env, path::Path};

fn main() {
    let cargo_manifest_dir = env::var("CARGO_MANIFEST_DIR").unwrap();
    let profile = env::var("PROFILE").unwrap();

    let icon_src = Path::new(&cargo_manifest_dir).join("grist.ico");
    let icon_dst = Path::new(&cargo_manifest_dir)
        .join("target")
        .join(&profile)
        .join("grist.ico");

    println!("cargo:rerun-if-changed={:?}", icon_dst);

    std::fs::copy(icon_src, icon_dst).unwrap();
}
