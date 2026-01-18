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
            if bindings.write_to_file(&output_file) {
                println!("Generated C bindings: {}", output_file);
            }
            else {
                eprintln!("Failed to write bindings to file");
                std::process::exit(1);
            }
        },
        Err(e) => {
            eprintln!("Failed to generate C bindings: {}", e);
            std::process::exit(1);
        },
    }
}

fn target_dir() -> PathBuf {
    let mut target = PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap());
    target.push("target");
    target.push(env::var("PROFILE").unwrap());
    target
}
