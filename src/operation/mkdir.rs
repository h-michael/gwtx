use crate::error::Result;

use std::path::Path;

/// Create a directory at the target path, including parent directories.
pub(crate) fn create_directory(target: &Path) -> Result<()> {
    if !target.exists() {
        std::fs::create_dir_all(target)?;
    }
    Ok(())
}

#[cfg(all(test, feature = "impure-test"))]
mod tests {
    use super::*;

    use tempfile::TempDir;

    #[test]
    fn test_create_directory() {
        let temp = TempDir::new().unwrap();
        let target = temp.path().join("new_dir");

        create_directory(&target).unwrap();

        assert!(target.exists());
        assert!(target.is_dir());
    }

    #[test]
    fn test_create_nested_directory() {
        let temp = TempDir::new().unwrap();
        let target = temp.path().join("a/b/c/d");

        create_directory(&target).unwrap();

        assert!(target.exists());
        assert!(target.is_dir());
    }

    #[test]
    fn test_create_existing_directory() {
        let temp = TempDir::new().unwrap();
        let target = temp.path().join("existing");

        std::fs::create_dir(&target).unwrap();

        // Should not error if directory already exists
        create_directory(&target).unwrap();

        assert!(target.exists());
    }
}
