fn main() {
    // Read project version from workspace VERSION file
    let manifest_dir = std::env::var("CARGO_MANIFEST_DIR").unwrap();
    let version_path = std::path::Path::new(&manifest_dir)
        .ancestors()
        .find_map(|p| {
            let candidate = p.join("VERSION");
            candidate.exists().then_some(candidate)
        })
        .expect("Could not find VERSION file in parent directories");

    let version = std::fs::read_to_string(&version_path)
        .expect("Failed to read VERSION file")
        .trim()
        .to_string();

    println!("cargo:rustc-env=CAPYDEPLOY_VERSION={version}");
    println!("cargo:rerun-if-changed={}", version_path.display());

    tauri_build::build()
}
