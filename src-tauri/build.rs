fn main() {
    // Re-run build.rs whenever .env changes
    println!("cargo:rerun-if-changed=.env");

    // Load .env and expose as compile-time env vars (GOOGLE_CLIENT_ID, GOOGLE_CLIENT_SECRET)
    let env_path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join(".env");
    if env_path.exists() {
        for item in dotenvy::from_path_iter(&env_path).expect("Failed to read .env") {
            let (key, value) = item.expect("Failed to parse .env entry");
            println!("cargo:rustc-env={}={}", key, value);
        }
    }

    tauri_build::build()
}
