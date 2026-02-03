use crate::error::{Error, Result};

use std::path::Path;

/// Copy a file or directory to target, creating parent dirs as needed.
pub(crate) fn copy_file(source: &Path, target: &Path) -> Result<()> {
    // Ensure parent directory exists
    if let Some(parent) = target.parent()
        && !parent.exists()
    {
        std::fs::create_dir_all(parent)?;
    }

    if source.is_dir() {
        copy_dir_recursive(source, target)?;
    } else {
        std::fs::copy(source, target).map_err(|e| Error::CopyFailed {
            source: source.to_path_buf(),
            target: target.to_path_buf(),
            cause: e,
        })?;
    }

    Ok(())
}

/// Recursively copy a directory
fn copy_dir_recursive(source: &Path, target: &Path) -> Result<()> {
    std::fs::create_dir_all(target)?;

    for entry in std::fs::read_dir(source)? {
        let entry = entry?;
        let source_path = entry.path();
        let target_path = target.join(entry.file_name());

        if source_path.is_dir() {
            copy_dir_recursive(&source_path, &target_path)?;
        } else {
            std::fs::copy(&source_path, &target_path).map_err(|e| Error::CopyFailed {
                source: source_path.clone(),
                target: target_path.clone(),
                cause: e,
            })?;
        }
    }

    Ok(())
}

#[cfg(all(test, feature = "impure-test"))]
mod tests {
    use super::*;

    use tempfile::TempDir;

    #[test]
    fn test_copy_file() {
        let temp = TempDir::new().unwrap();
        let source = temp.path().join("source.txt");
        let target = temp.path().join("target.txt");

        std::fs::write(&source, "hello").unwrap();

        copy_file(&source, &target).unwrap();

        assert!(target.exists());
        assert_eq!(std::fs::read_to_string(&target).unwrap(), "hello");
    }

    #[test]
    fn test_copy_file_creates_parent_dirs() {
        let temp = TempDir::new().unwrap();
        let source = temp.path().join("source.txt");
        let target = temp.path().join("nested/dir/target.txt");

        std::fs::write(&source, "hello").unwrap();

        copy_file(&source, &target).unwrap();

        assert!(target.exists());
    }

    #[test]
    fn test_copy_directory() {
        let temp = TempDir::new().unwrap();
        let source_dir = temp.path().join("source_dir");
        let target_dir = temp.path().join("target_dir");

        std::fs::create_dir(&source_dir).unwrap();
        std::fs::write(source_dir.join("file1.txt"), "content1").unwrap();
        std::fs::create_dir(source_dir.join("subdir")).unwrap();
        std::fs::write(source_dir.join("subdir/file2.txt"), "content2").unwrap();

        copy_file(&source_dir, &target_dir).unwrap();

        assert!(target_dir.join("file1.txt").exists());
        assert!(target_dir.join("subdir/file2.txt").exists());
        assert_eq!(
            std::fs::read_to_string(target_dir.join("file1.txt")).unwrap(),
            "content1"
        );
    }
}
