use std::path::{Path, PathBuf};

use tokio::fs as tokio_fs;

use crate::{Collection, Result};

pub struct Store {
    root_path: PathBuf,
}

impl Store {
    pub async fn new(root_path: impl AsRef<Path>) -> Result<Self> {
        let root_path = root_path.as_ref().to_path_buf();
        tokio_fs::create_dir_all(&root_path).await?;
        Ok(Self {
            root_path,
        })
    }

    pub async fn collection(&self, name: &str) -> Result<Collection> {
        let path = self.root_path.join("data").join(name);
        tokio_fs::create_dir_all(&path).await?;
        Ok(Collection {
            name: name.to_string(),
            path,
        })
    }
}

#[cfg(test)]
mod tests {
    use tempfile::tempdir;

    use super::*;

    #[tokio::test]
    async fn test_store_new_creates_directory() {
        let temp_dir = tempdir().unwrap();
        let store_path = temp_dir.path().join("store");

        let _store = Store::new(&store_path).await.unwrap();
        assert!(store_path.exists());
        assert!(store_path.is_dir());
    }

    #[tokio::test]
    async fn test_store_new_with_existing_directory() {
        let temp_dir = tempdir().unwrap();
        let store_path = temp_dir.path();

        // Directory already exists
        let _store = Store::new(&store_path).await.unwrap();
        assert!(store_path.exists());
    }

    #[tokio::test]
    async fn test_store_collection_creates_subdirectory() {
        let temp_dir = tempdir().unwrap();
        let store = Store::new(temp_dir.path()).await.unwrap();

        let collection = store.collection("users").await.unwrap();
        assert!(collection.path.exists());
        assert!(collection.path.is_dir());
        assert_eq!(collection.name, "users");
    }

    #[tokio::test]
    async fn test_store_collection_with_special_characters() {
        let temp_dir = tempdir().unwrap();
        let store = Store::new(temp_dir.path()).await.unwrap();

        let collection = store.collection("user_data-123").await.unwrap();
        assert!(collection.path.exists());
        assert_eq!(collection.name, "user_data-123");
    }

    #[tokio::test]
    async fn test_store_collection_multiple_calls() {
        let temp_dir = tempdir().unwrap();
        let store = Store::new(temp_dir.path()).await.unwrap();

        let coll1 = store.collection("users").await.unwrap();
        let coll2 = store.collection("users").await.unwrap();

        assert_eq!(coll1.name, coll2.name);
        assert_eq!(coll1.path, coll2.path);
    }
}
