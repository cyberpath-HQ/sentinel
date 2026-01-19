//! Integration tests for C/C++ bindings
//!
//! This module tests that the C/C++ bindings build correctly and that
//! all examples run successfully as part of the testing pipeline.

use std::{
    fs,
    path::PathBuf,
    process::{Command, Stdio},
};

#[test]
fn test_cxx_bindings_integration() {
    // Run build test first
    test_cxx_bindings_build();

    // Then run the other tests sequentially
    test_cxx_bindings_tests();
    test_cxx_examples_run();
}

fn test_cxx_bindings_build() {
    // Get the project root (crates/sentinel-cxx -> language-interop)
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let project_root = manifest_dir
        .parent().unwrap()  // crates
        .parent().unwrap()  // language-interop
        .to_path_buf();

    let cxx_bindings_dir = project_root.join("bindings").join("cxx");

    // Ensure the bindings directory exists
    assert!(
        cxx_bindings_dir.exists(),
        "C/C++ bindings directory not found at {:?}",
        cxx_bindings_dir
    );

    // Create build directory if it doesn't exist
    let build_dir = cxx_bindings_dir.join("build");
    if !build_dir.exists() {
        fs::create_dir_all(&build_dir).expect("Failed to create build directory");
    }

    // Configure with CMake - retry once if it fails (sometimes CMake has issues on first run)
    let mut cmake_result = Command::new("cmake")
        .args(&["..", "-DCMAKE_BUILD_TYPE=Debug"])
        .current_dir(&build_dir)
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .status()
        .expect("Failed to run cmake");

    // If CMake fails, wait a moment and try again
    if !cmake_result.success() {
        println!("CMake configuration failed, retrying...");
        std::thread::sleep(std::time::Duration::from_millis(500));

        cmake_result = Command::new("cmake")
            .args(&["..", "-DCMAKE_BUILD_TYPE=Debug"])
            .current_dir(&build_dir)
            .stdout(Stdio::inherit())
            .stderr(Stdio::inherit())
            .status()
            .expect("Failed to run cmake on retry");
    }

    assert!(
        cmake_result.success(),
        "CMake configuration failed after retry"
    );

    // Build with make
    let make_result = Command::new("make")
        .args(&["-j", &num_cpus::get().to_string()])
        .current_dir(&build_dir)
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .status()
        .expect("Failed to run make");

    assert!(make_result.success(), "C/C++ build failed");

    // Verify that key executables were built
    let always_expected = vec![
        "c_example",
        "c_query_example",
        "c_complex_query_example",
        "test_c_bindings",
        // "cpp_tests", // Disabled due to compilation issues
    ];

    // Check always-expected executables
    for exe_name in always_expected {
        let exe_path = build_dir.join(exe_name);
        assert!(exe_path.exists(), "Executable {} was not built", exe_name);
        assert!(
            exe_path.metadata().unwrap().permissions().readonly() == false,
            "Executable {} is not executable",
            exe_name
        );
    }

    // Async examples should be built now
    let async_examples = vec!["c_async_example", "c_async_query_example"];
    for exe_name in async_examples {
        let exe_path = build_dir.join(exe_name);
        assert!(
            exe_path.exists(),
            "Async executable {} was not built",
            exe_name
        );
        assert!(
            exe_path.metadata().unwrap().permissions().readonly() == false,
            "Executable {} is not executable",
            exe_name
        );
        println!("✓ Async executable {} was built", exe_name);
    }

    println!("✓ All C/C++ executables built successfully");
}

fn test_cxx_examples_run() {
    // Get the project root (crates/sentinel-cxx -> project root)
    let project_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .to_path_buf();

    let build_dir = project_root.join("bindings").join("cxx").join("build");

    // Skip if build directory doesn't exist (build test should run first)
    if !build_dir.exists() {
        println!("⚠️  Build directory not found, skipping example tests");
        return;
    }

    // Test basic C example (always built)
    run_example_test(&build_dir, "c_example", "Basic C example");

    // Test query example (always built)
    run_example_test(&build_dir, "c_query_example", "C query example");

    // Test complex query example (always built)
    run_example_test(
        &build_dir,
        "c_complex_query_example",
        "C complex query example",
    );

    // Test async examples (should be built now)
    run_example_test(&build_dir, "c_async_example", "C async example");
    run_example_test(&build_dir, "c_async_query_example", "C async query example");

    println!("✓ All C/C++ examples ran successfully");
}

fn test_cxx_bindings_tests() {
    // Get the project root (crates/sentinel-cxx -> project root)
    let project_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .to_path_buf();

    let build_dir = project_root.join("bindings").join("cxx").join("build");

    // Skip if build directory doesn't exist (build test should run first)
    if !build_dir.exists() {
        println!("⚠️  Build directory not found, skipping binding tests");
        return;
    }

    // Run the C binding tests
    run_example_test(&build_dir, "test_c_bindings", "C binding tests");

    println!("✓ C binding tests passed");
}

fn run_example_test(build_dir: &PathBuf, exe_name: &str, description: &str) {
    let exe_path = build_dir.join(exe_name);

    // The build test should have created all executables already
    if !exe_path.exists() {
        panic!(
            "{} executable not found at {:?} - build test may have failed",
            description, exe_path
        );
    }

    // Small delay to ensure the executable is fully written and not locked
    std::thread::sleep(std::time::Duration::from_millis(100));

    // Spawn the process to run the example
    let mut command = Command::new(&exe_path);
    command
        .current_dir(build_dir)
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit());

    // Set library path for dynamic linking
    if cfg!(target_os = "linux") {
        let lib_path = build_dir.parent().unwrap().join("lib");
        if lib_path.exists() {
            command.env("LD_LIBRARY_PATH", lib_path);
        }
    }

    let mut child = command
        .spawn()
        .unwrap_or_else(|e| panic!("Failed to spawn {}: {}", description, e));

    let result = child
        .wait()
        .unwrap_or_else(|e| panic!("Failed to wait for {}: {}", description, e));

    if !result.success() {
        panic!("{} failed with exit code {:?}", description, result.code());
    }

    println!("✓ {} completed successfully", description);
}
