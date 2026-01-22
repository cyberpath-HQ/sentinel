use std::{env, path::PathBuf};

fn main() {
    let crate_dir = env::var("CARGO_MANIFEST_DIR").unwrap();
    let package_name = env::var("CARGO_PKG_NAME").unwrap();
    let output_file = target_dir()
        .join(format!("{}.h", package_name))
        .display()
        .to_string();

    match cbindgen::generate(crate_dir) {
        Ok(bindings) => {
            // Try to write the file, but don't fail the build if it doesn't work
            // The C bindings will still be available at runtime
            let result = bindings.write_to_file(&output_file);
            if result {
                println!("Generated C bindings: {}", output_file);
            }
            else {
                println!(
                    "Warning: Could not write C bindings to file: {}",
                    output_file
                );
                println!("C bindings will still be available at runtime");
            }
        },
        Err(e) => {
            println!("ERROR: Failed to generate C bindings: {}", e);
            std::process::exit(1);
        },
    }
}

/// Returns the target directory path for the build.
fn target_dir() -> PathBuf {
    PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap())
        .join("target")
        .join(env::var("PROFILE").unwrap())
}
