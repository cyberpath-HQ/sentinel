#[cfg(test)]
mod nodejs_integration_tests {
    use std::{
        env,
        path::PathBuf,
        process::{Command, Stdio},
    };

    fn bindings_js_path() -> PathBuf {
        let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        manifest_dir
            .parent()  // crates/
            .unwrap()
            .parent()  // language-interop/
            .unwrap()
            .join("bindings/js")
    }

    fn native_bindings_exist() -> bool {
        let base_path = bindings_js_path().join("native");
        base_path.join("sentinel_js.node").exists() ||
            base_path.join("sentinel_js.dll").exists() ||
            base_path.join("sentinel_js.dylib").exists()
    }

    fn node_available() -> bool {
        Command::new("node")
            .arg("--version")
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false)
    }

    #[test]
    fn test_node_available() {
        assert!(
            node_available(),
            "Node.js must be installed. Install from https://nodejs.org/"
        );
    }

    #[test]
    fn test_native_bindings_exist() {
        let bindings_path = bindings_js_path();
        let native_path = bindings_path.join("native");

        if !native_path.exists() {
            std::fs::create_dir_all(&native_path).expect("Failed to create native directory");
        }

        if !native_bindings_exist() {
            println!("Building native bindings...");
            let workspace_root = bindings_path.parent().unwrap().parent().unwrap();

            let build_result = Command::new("cargo")
                .args(&["build", "--release", "-p", "sentinel-js"])
                .current_dir(workspace_root)
                .stdout(Stdio::piped())
                .stderr(Stdio::piped())
                .output()
                .expect("Failed to run cargo build");

            if !build_result.status.success() {
                let stderr = String::from_utf8_lossy(&build_result.stderr);
                panic!("cargo build failed: {}", stderr);
            }

            println!("Native bindings built successfully");

            // Find and copy the built library
            let release_path = workspace_root.join("target/release");

            let lib_name = if cfg!(target_os = "linux") {
                "libsentinel_js.so"
            }
            else if cfg!(target_os = "macos") {
                "libsentinel_js.dylib"
            }
            else if cfg!(target_os = "windows") {
                "sentinel_js.dll"
            }
            else {
                panic!("Unsupported platform");
            };

            let src_path = release_path.join(lib_name);
            let dest_path = native_path.join("sentinel_js.node");

            if src_path.exists() {
                std::fs::copy(&src_path, &dest_path).expect(&format!(
                    "Failed to copy native library from {:?}",
                    src_path
                ));
            }
            else {
                // Try deps folder
                let deps_path = release_path.join("deps").join(lib_name);
                if deps_path.exists() {
                    std::fs::copy(&deps_path, &dest_path).expect(&format!(
                        "Failed to copy native library from {:?}",
                        deps_path
                    ));
                }
                else {
                    panic!(
                        "Could not find built library at {:?} or {:?}",
                        src_path, deps_path
                    );
                }
            }

            println!("Native bindings copied to {:?}", dest_path);
        }

        assert!(
            native_bindings_exist(),
            "Native bindings must exist at {}",
            native_path.display()
        );
    }

    #[test]
    fn test_npm_dependencies() {
        let bindings_path = bindings_js_path();
        let node_modules_path = bindings_path.join("node_modules");
        let mocha_path = node_modules_path.join(".bin/mocha");

        if !node_modules_path.exists() || !mocha_path.exists() {
            println!("Installing npm dependencies...");
            let install_result = Command::new("npm")
                .args(&["install", "--no-audit", "--no-fund", "--loglevel=error"])
                .current_dir(&bindings_path)
                .stdout(Stdio::piped())
                .stderr(Stdio::piped())
                .output()
                .expect("Failed to run npm install");

            if !install_result.status.success() {
                let stderr = String::from_utf8_lossy(&install_result.stderr);
                // Try again with clean install
                println!("First npm install failed, retrying with clean install...");
                std::fs::remove_dir_all(&node_modules_path).ok();
                std::fs::remove_dir_all(&bindings_path.join("package-lock.json")).ok();

                let retry_result = Command::new("npm")
                    .args(&["install", "--no-audit", "--no-fund", "--loglevel=error"])
                    .current_dir(&bindings_path)
                    .stdout(Stdio::piped())
                    .stderr(Stdio::piped())
                    .output()
                    .expect("Failed to run npm install");

                if !retry_result.status.success() {
                    let retry_stderr = String::from_utf8_lossy(&retry_result.stderr);
                    panic!("npm install failed: {}\n\n{}", stderr, retry_stderr);
                }
            }
            println!("npm dependencies installed");
        }

        assert!(
            mocha_path.exists(),
            "mocha must be installed at {}",
            mocha_path.display()
        );
    }

    #[test]
    fn test_js_integration_tests() {
        // Ensure dependencies are available
        test_native_bindings_exist();
        test_npm_dependencies();

        let bindings_path = bindings_js_path();
        let test_script = bindings_path.join("tests/integration.js");
        let mocha_path = bindings_path.join("node_modules/.bin/mocha");

        assert!(
            test_script.exists(),
            "JavaScript test script must exist at {}",
            test_script.display()
        );

        let output = Command::new("node")
            .arg(&mocha_path)
            .arg(&test_script)
            .current_dir(&bindings_path)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()
            .expect("Failed to execute mocha tests");

        let stdout = String::from_utf8_lossy(&output.stdout);
        let stderr = String::from_utf8_lossy(&output.stderr);

        assert!(
            output.status.success(),
            "JavaScript tests failed:\n\nSTDOUT:\n{}\n\nSTDERR:\n{}\n",
            stdout,
            stderr
        );

        assert!(
            stdout.contains("passing") || stdout.contains("pending"),
            "Expected test output, got: {}",
            stdout
        );
    }

    #[test]
    fn test_js_examples_compile() {
        test_node_available();

        let js_path = bindings_js_path();
        let example_files = vec!["example.js", "example-esm.mjs"];

        for example in example_files {
            let example_path = js_path.join(example);
            if example_path.exists() {
                let output = Command::new("node")
                    .arg("--check")
                    .arg(&example_path)
                    .stdout(Stdio::piped())
                    .stderr(Stdio::piped())
                    .output()
                    .expect("Failed to check example syntax");

                assert!(
                    output.status.success(),
                    "Example {} has syntax errors: {}",
                    example,
                    String::from_utf8_lossy(&output.stderr)
                );
            }
        }
    }
}
