use crate::error::{Error, Result};

use std::path::Path;

/// Create a symlink at target pointing to source.
pub(crate) fn create_symlink(source: &Path, target: &Path) -> Result<()> {
    // Ensure parent directory exists
    if let Some(parent) = target.parent() {
        if !parent.exists() {
            std::fs::create_dir_all(parent)?;
        }
    }

    #[cfg(unix)]
    {
        std::os::unix::fs::symlink(source, target).map_err(|e| Error::SymlinkFailed {
            source: source.to_path_buf(),
            target: target.to_path_buf(),
            cause: e,
        })?;
    }

    #[cfg(windows)]
    {
        let result = if source.is_dir() {
            std::os::windows::fs::symlink_dir(source, target)
        } else {
            std::os::windows::fs::symlink_file(source, target)
        };

        result.map_err(|e| {
            // Check for permission error on Windows
            if e.raw_os_error() == Some(1314) {
                Error::WindowsSymlinkPermission
            } else {
                Error::SymlinkFailed {
                    source: source.to_path_buf(),
                    target: target.to_path_buf(),
                    cause: e,
                }
            }
        })?;
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    use tempfile::TempDir;

    #[test]
    fn test_create_symlink_file() {
        let temp = TempDir::new().unwrap();
        let source = temp.path().join("source.txt");
        let target = temp.path().join("target.txt");

        std::fs::write(&source, "hello").unwrap();

        create_symlink(&source, &target).unwrap();

        assert!(target.is_symlink());
        assert_eq!(std::fs::read_to_string(&target).unwrap(), "hello");
    }

    #[test]
    fn test_create_symlink_creates_parent_dirs() {
        let temp = TempDir::new().unwrap();
        let source = temp.path().join("source.txt");
        let target = temp.path().join("nested/dir/target.txt");

        std::fs::write(&source, "hello").unwrap();

        create_symlink(&source, &target).unwrap();

        assert!(target.is_symlink());
    }
}
