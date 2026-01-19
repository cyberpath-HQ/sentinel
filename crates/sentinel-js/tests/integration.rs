#[cfg(test)]
mod sentinel_js_tests {
    #[test]
    fn test_crate_compiles() {
        // This test just verifies the crate compiles successfully
        let _: i32 = 0;
    }

    #[test]
    fn test_json_serialization() {
        use serde_json::Value;

        let data = serde_json::json!({
            "name": "Test",
            "value": 42,
            "active": true
        });

        assert_eq!(data["name"], "Test");
        assert_eq!(data["value"], 42);
        assert_eq!(data["active"], true);
    }

    #[test]
    fn test_path_buffer_operations() {
        use std::path::PathBuf;

        let mut path = PathBuf::new();
        path.push("/data");
        path.push("collections");
        path.push("documents");

        assert_eq!(path.to_string_lossy(), "/data/collections/documents");
    }

    #[test]
    fn test_tempdir_creation() {
        let path = tempfile::Builder::new()
            .prefix("sentinel_test")
            .tempdir()
            .expect("Failed to create temp directory")
            .into_path();

        assert!(path.exists());

        std::fs::remove_dir_all(&path).expect("Failed to remove temp directory");
        assert!(!path.exists());
    }

    #[test]
    fn test_async_runtime_creation() {
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_time()
            .build()
            .expect("Failed to create runtime");

        rt.block_on(async {
            tokio::time::sleep(std::time::Duration::from_millis(1)).await;
        });
    }
}
