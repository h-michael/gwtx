//! Hook trust management system
//!
//! This module manages whether git hooks in `.gwtx.yaml` have been explicitly reviewed
//! and trusted by the user. Trust is based on a SHA256 hash of the hook commands and
//! descriptions, combined with the primary worktree path.
//!
//! ## Design Overview
//!
//! **Primary Worktree Path Usage**: Uses the primary (main) worktree path for hashing instead
//! of the current repository path. This allows all worktrees created from the same main
//! worktree to share the same trust state. When a user creates multiple worktrees (e.g.,
//! feature branches), they all trust the same hooks without re-approval.
//!
//! **Nested Directory Structure**: Trust files are stored in directories named after the
//! primary worktree path (e.g., `~/.local/share/gwtx/trusted/-foo-bar-baz/{hash}.yaml`).
//! This structure provides performance benefits: when many repositories are trusted, the
//! nested directory avoids scanning hundreds of files in a single flat directory.
//!
//! **Automatic Cleanup of Old Files**: When hooks are re-trusted with new content,
//! all previous trust files for that primary worktree are deleted. This prevents
//! reversion attacks where an attacker could restore old `.gwtx.yaml` and use previously
//! trusted hooks. Only the current hooks hash is valid for a given primary worktree.
//!
//! **Path Verification**: The main_worktree_path is stored in the trust file and
//! verified during `is_trusted()`. This prevents symlink attacks where an attacker could
//! replace the worktree directory with a symlink to a different path with old trusted hooks.
//!
//! **Empty Hooks Are Implicitly Trusted**: If `.gwtx.yaml` has no hooks (or is empty),
//! `is_trusted()` returns true immediately without checking disk. Rationale: no hooks means
//! nothing to trust; the user cannot create a `.gwtx.yaml` without hooks and then claim
//! hooks exist.

use crate::config::Hooks;
use crate::error::{Error, Result};

use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use chrono::Utc;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

#[cfg(test)]
use crate::config::HookEntry;

const TRUST_DIR_NAME: &str = "gwtx/trusted";

/// Represents a trusted hook configuration.
///
/// Fields:
/// - `main_worktree_path`: The main worktree path (where `.git` is a directory).
///   All worktrees created from this path share the same trust. Stored for verification
///   during trust checks to prevent symlink attacks.
/// - `trusted_at`: RFC3339 timestamp of when the hooks were trusted.
#[derive(Debug, Serialize, Deserialize)]
pub(crate) struct TrustEntry {
    pub main_worktree_path: PathBuf,
    pub trusted_at: String,
}

/// Get trust storage directory
/// Uses XDG_DATA_HOME or falls back to ~/.local/share on Linux
fn trust_dir() -> Result<PathBuf> {
    let base = dirs::data_dir().ok_or(Error::TrustStorageNotFound)?;
    Ok(base.join(TRUST_DIR_NAME))
}

/// Compute SHA256 hash of hooks configuration.
///
/// Combines the main worktree path and all hook commands/descriptions into a single
/// hash. This hash is used as the trust file identifier. If either the path or any hook
/// content changes, the hash changes, requiring re-trust.
///
/// Hash includes:
/// - Canonicalized main_worktree_path (prevents different path representations)
/// - Hook commands and descriptions from all phases (pre_add, post_add, pre_remove, post_remove)
///
/// **Stability**: Uses explicit JSON serialization to ensure the hash remains stable
/// across Rust compiler versions. Debug trait output format isn't guaranteed stable,
/// so we use JSON's stable text representation instead.
///
/// Returns the hash as a lowercase hex string (64 hex characters for SHA256).
pub(crate) fn compute_hash(main_worktree_path: &Path, hooks: &Hooks) -> Result<String> {
    let mut hasher = Sha256::new();

    // Canonicalize to ensure stable representation across systems
    let canonical_path =
        main_worktree_path
            .canonicalize()
            .map_err(|e| Error::TrustVerificationFailed {
                message: format!("Failed to canonicalize worktree path: {}", e),
            })?;

    // Use JSON serialization for stable, version-independent format
    // Serialize: { path: "...", hooks: { pre_add: [...], post_add: [...], ... } }
    let path_str = canonical_path.to_string_lossy();
    hasher.update(path_str.as_bytes());
    hasher.update(b"\n");

    // Serialize hooks as JSON for stable representation
    let hooks_json = serde_json::to_string(hooks).map_err(|e| Error::TrustFileSerialization {
        message: format!("Failed to serialize hooks: {}", e),
    })?;
    hasher.update(hooks_json.as_bytes());

    Ok(format!("{:x}", hasher.finalize()))
}

/// Convert main worktree path to directory name for nested storage.
///
/// Hashes the path to create an OS-independent, filesystem-safe directory name.
/// Uses SHA256 hash of the path (first 16 hex chars) to avoid OS-specific invalid characters.
///
/// Example outputs:
/// - "/home/user/myrepo" → "a1b2c3d4e5f6g7h8"
/// - "C:\Users\user\repo" → "f8e7d6c5b4a39291"
///
/// Purpose: Organize trust files into subdirectories per main worktree path while
/// ensuring compatibility across Unix and Windows filesystems.
/// Performance benefit: Prevents a single directory from containing thousands of files
/// when many repositories are trusted. Instead, each main worktree gets its own directory.
fn main_worktree_dir_name(path: &Path) -> String {
    use sha2::{Digest, Sha256};

    let path_str = path.to_string_lossy();
    let mut hasher = Sha256::new();
    hasher.update(path_str.as_bytes());
    let hash = hasher.finalize();

    // Use first 16 hex characters for reasonable uniqueness while keeping directory names shorter
    format!("{:x}", hash)[..16].to_string()
}

/// Check if hooks are trusted for the given main worktree.
///
/// Returns true if:
/// 1. No hooks are defined (empty configuration)
/// 2. A trust file exists with a matching hash AND the stored main_worktree_path matches
///
/// Returns false if:
/// 1. Hooks exist but no trust file found
/// 2. Trust file exists but main_worktree_path doesn't match (symlink attack protection)
/// 3. Trust file is corrupted
///
/// **TOCTOU Safety**: Directly attempts to read the file without pre-checking existence.
/// "File not found" errors are treated as Ok(false) for atomicity. If the file is deleted
/// between check and read, we get the correct result (not trusted).
///
/// **Path Verification**: Prevents symlink attacks where an attacker could replace the
/// worktree with a symlink to a different location containing old trusted hooks.
pub(crate) fn is_trusted(main_worktree_path: &Path, hooks: &Hooks) -> Result<bool> {
    if !hooks.has_hooks() {
        return Ok(true); // No hooks = implicitly trusted
    }

    // Canonicalize path - fail if it doesn't exist, ensuring consistent behavior
    let canonical_path =
        main_worktree_path
            .canonicalize()
            .map_err(|e| Error::TrustVerificationFailed {
                message: format!("Failed to canonicalize worktree path: {}", e),
            })?;

    let hash = compute_hash(&canonical_path, hooks)?;
    let dir_name = main_worktree_dir_name(&canonical_path);
    let trust_file = trust_dir()?.join(&dir_name).join(format!("{}.yaml", hash));

    // TOCTOU-safe: Attempt to read file directly instead of checking existence first.
    // If file is deleted between our check and read, we get correct result (not trusted).
    let content = match fs::read_to_string(&trust_file) {
        Ok(content) => content,
        Err(e) if e.kind() == io::ErrorKind::NotFound => return Ok(false),
        Err(e) => return Err(e.into()),
    };

    // Verify stored main_worktree_path matches current path
    let entry: TrustEntry =
        serde_yaml::from_str(&content).map_err(|e| Error::TrustFileCorrupted {
            message: e.to_string(),
        })?;

    if entry.main_worktree_path != canonical_path {
        return Ok(false);
    }

    Ok(true)
}

/// Trust hooks for a repository by marking them as reviewed and approved.
///
/// Creates a trust file at `~/.local/share/gwtx/trusted/{primary-worktree-dir}/{hash}.yaml`
/// containing the main_worktree_path and timestamp.
///
/// **Automatic Cleanup**: Deletes all old trust files in the same directory before creating
/// the new one. This prevents reversion attacks where an attacker could restore an old
/// `.gwtx.yaml` and use previously trusted hooks with different content.
///
/// Reversion Attack Scenario (prevented by this cleanup):
/// 1. User trusts hooks (file hash=ABC123 created)
/// 2. Attacker modifies `.gwtx.yaml` with dangerous commands
/// 3. User runs `gwtx trust` again with new content (file hash=XYZ789 created)
/// 4. Attacker reverts `.gwtx.yaml` to original content (hash=ABC123)
/// 5. OLD BEHAVIOR: hash=ABC123 still exists, hooks trusted without re-review
/// 6. NEW BEHAVIOR: hash=ABC123 was deleted in step 3, reversion fails
pub(crate) fn trust(main_worktree_path: &Path, hooks: &Hooks) -> Result<()> {
    if !hooks.has_hooks() {
        return Ok(());
    }

    // Canonicalize path - fail if it doesn't exist
    let canonical_path =
        main_worktree_path
            .canonicalize()
            .map_err(|e| Error::TrustVerificationFailed {
                message: format!("Failed to canonicalize worktree path: {}", e),
            })?;

    let trust_base_path = trust_dir()?;
    let dir_name = main_worktree_dir_name(&canonical_path);
    let trust_repo_path = trust_base_path.join(&dir_name);
    fs::create_dir_all(&trust_repo_path)?;

    // Remove old trust files for this main_worktree_path (before saving new one).
    // Security: Prevents reversion attacks by ensuring only the current hooks hash
    // is valid for this primary worktree. If .gwtx.yaml is reverted to an older
    // version, the old hash file won't exist and re-trust will be required.
    if trust_repo_path.exists() {
        for entry in fs::read_dir(&trust_repo_path)?.flatten() {
            let path = entry.path();
            if path.extension().is_some_and(|ext| ext == "yaml") {
                let _ = fs::remove_file(&path);
            }
        }
    }

    let hash = compute_hash(&canonical_path, hooks)?;
    let trust_file = trust_repo_path.join(format!("{}.yaml", hash));

    let entry = TrustEntry {
        main_worktree_path: canonical_path,
        trusted_at: Utc::now().to_rfc3339(),
    };

    let content = serde_yaml::to_string(&entry).map_err(|e| Error::TrustFileSerialization {
        message: e.to_string(),
    })?;

    fs::write(&trust_file, content)?;

    Ok(())
}

/// Remove trust for hooks of a repository.
///
/// Deletes the trust file matching the current hooks hash. Returns true if a trust file
/// existed and was deleted, false if no trust file was found for these hooks.
pub(crate) fn untrust(main_worktree_path: &Path, hooks: &Hooks) -> Result<bool> {
    if !hooks.has_hooks() {
        return Ok(false);
    }

    // Canonicalize path - fail if it doesn't exist
    let canonical_path =
        main_worktree_path
            .canonicalize()
            .map_err(|e| Error::TrustVerificationFailed {
                message: format!("Failed to canonicalize worktree path: {}", e),
            })?;

    let hash = compute_hash(&canonical_path, hooks)?;
    let dir_name = main_worktree_dir_name(&canonical_path);
    let trust_file = trust_dir()?.join(&dir_name).join(format!("{}.yaml", hash));

    if trust_file.exists() {
        fs::remove_file(&trust_file)?;
        Ok(true)
    } else {
        Ok(false)
    }
}

/// List all trusted repository hooks.
///
/// Traverses the nested directory structure at `~/.local/share/gwtx/trusted/`
/// and returns all TrustEntry objects found. Each directory under `trusted/` represents
/// a different main_worktree_path, and each `.yaml` file within that directory represents
/// a different trusted hook configuration.
///
/// Returns an empty vector if no trust files exist.
/// Logs warnings to stderr for corrupted or unreadable files, but continues processing.
pub(crate) fn list_trusted() -> Result<Vec<TrustEntry>> {
    let trust_path = trust_dir()?;

    if !trust_path.exists() {
        return Ok(Vec::new());
    }

    let mut entries = Vec::new();
    for dir_entry in fs::read_dir(&trust_path)?.flatten() {
        let dir_path = dir_entry.path();
        if dir_path.is_dir() {
            for file_entry in fs::read_dir(&dir_path)?.flatten() {
                let file_path = file_entry.path();
                if file_path.extension().is_some_and(|e| e == "yaml") {
                    match fs::read_to_string(&file_path) {
                        Ok(content) => match serde_yaml::from_str::<TrustEntry>(&content) {
                            Ok(trust_entry) => entries.push(trust_entry),
                            Err(e) => {
                                eprintln!(
                                    "Warning: Failed to parse trust file: {}\n         Error: {}",
                                    file_path.display(),
                                    e
                                );
                            }
                        },
                        Err(e) => {
                            eprintln!(
                                "Warning: Failed to read trust file: {}\n         Error: {}",
                                file_path.display(),
                                e
                            );
                        }
                    }
                }
            }
        }
    }

    Ok(entries)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_compute_hash_empty_hooks() {
        let temp_dir = TempDir::new().unwrap();
        let hooks = Hooks::default();

        let hash = compute_hash(temp_dir.path(), &hooks).unwrap();
        assert!(!hash.is_empty());
        assert_eq!(hash.len(), 64); // SHA256 produces 64 hex characters
    }

    #[test]
    fn test_compute_hash_different_hooks() {
        let temp_dir = TempDir::new().unwrap();

        let hooks1 = Hooks {
            pre_add: vec![HookEntry {
                command: "echo 'test1'".to_string(),
                description: None,
            }],
            ..Default::default()
        };

        let hooks2 = Hooks {
            pre_add: vec![HookEntry {
                command: "echo 'test2'".to_string(),
                description: None,
            }],
            ..Default::default()
        };

        let hash1 = compute_hash(temp_dir.path(), &hooks1).unwrap();
        let hash2 = compute_hash(temp_dir.path(), &hooks2).unwrap();

        assert_ne!(hash1, hash2);
    }

    #[test]
    fn test_is_trusted_empty_hooks() {
        let temp_dir = TempDir::new().unwrap();
        let hooks = Hooks::default();

        assert!(is_trusted(temp_dir.path(), &hooks).unwrap());
    }

    #[test]
    fn test_trust_and_untrust() {
        let temp_dir = TempDir::new().unwrap();
        let hooks = Hooks {
            pre_add: vec![HookEntry {
                command: "echo 'test'".to_string(),
                description: None,
            }],
            ..Default::default()
        };

        // Initially not trusted
        assert!(!is_trusted(temp_dir.path(), &hooks).unwrap());

        // Trust the hooks
        trust(temp_dir.path(), &hooks).unwrap();
        assert!(is_trusted(temp_dir.path(), &hooks).unwrap());

        // Untrust the hooks
        assert!(untrust(temp_dir.path(), &hooks).unwrap());
        assert!(!is_trusted(temp_dir.path(), &hooks).unwrap());

        // Untrusting again should return false
        assert!(!untrust(temp_dir.path(), &hooks).unwrap());
    }

    #[test]
    fn test_list_trusted_no_error() {
        // Just ensure list_trusted() doesn't error
        // Length may vary depending on system state
        let _entries = list_trusted().unwrap();
    }

    #[test]
    fn test_list_trusted_with_trusted_repo() {
        let temp_dir = TempDir::new().unwrap();
        let hooks = Hooks {
            pre_add: vec![HookEntry {
                command: "echo 'test'".to_string(),
                description: None,
            }],
            post_add: vec![HookEntry {
                command: "npm install".to_string(),
                description: None,
            }],
            ..Default::default()
        };

        // Trust the hooks
        trust(temp_dir.path(), &hooks).unwrap();

        // List should include this repo
        let entries = list_trusted().unwrap();
        let canonical_path = temp_dir
            .path()
            .canonicalize()
            .unwrap_or_else(|_| temp_dir.path().to_path_buf());

        let found = entries
            .iter()
            .any(|e| e.main_worktree_path == canonical_path);
        assert!(found, "Trusted repository should be in the list");

        // Cleanup
        untrust(temp_dir.path(), &hooks).unwrap();
    }

    #[test]
    fn test_list_trusted_multiple_repos() {
        let temp_dir1 = TempDir::new().unwrap();
        let temp_dir2 = TempDir::new().unwrap();

        let hooks1 = Hooks {
            pre_add: vec![HookEntry {
                command: "echo 'repo1'".to_string(),
                description: None,
            }],
            ..Default::default()
        };

        let hooks2 = Hooks {
            pre_add: vec![HookEntry {
                command: "echo 'repo2'".to_string(),
                description: None,
            }],
            ..Default::default()
        };

        // Trust both repos
        trust(temp_dir1.path(), &hooks1).unwrap();
        trust(temp_dir2.path(), &hooks2).unwrap();

        // List should include both
        let entries = list_trusted().unwrap();
        let canonical_path1 = temp_dir1
            .path()
            .canonicalize()
            .unwrap_or_else(|_| temp_dir1.path().to_path_buf());
        let canonical_path2 = temp_dir2
            .path()
            .canonicalize()
            .unwrap_or_else(|_| temp_dir2.path().to_path_buf());

        let found1 = entries
            .iter()
            .any(|e| e.main_worktree_path == canonical_path1);
        let found2 = entries
            .iter()
            .any(|e| e.main_worktree_path == canonical_path2);

        assert!(found1, "First trusted repository should be in the list");
        assert!(found2, "Second trusted repository should be in the list");

        // Cleanup
        untrust(temp_dir1.path(), &hooks1).unwrap();
        untrust(temp_dir2.path(), &hooks2).unwrap();
    }

    #[test]
    fn test_trust_entry_contains_main_worktree_path() {
        let temp_dir = TempDir::new().unwrap();
        let hooks = Hooks {
            pre_add: vec![HookEntry {
                command: "echo 'pre'".to_string(),
                description: None,
            }],
            post_add: vec![HookEntry {
                command: "npm install".to_string(),
                description: None,
            }],
            pre_remove: vec![HookEntry {
                command: "echo 'cleanup'".to_string(),
                description: None,
            }],
            post_remove: vec![HookEntry {
                command: "./scripts/cleanup.sh".to_string(),
                description: None,
            }],
        };

        // Trust the hooks
        trust(temp_dir.path(), &hooks).unwrap();

        // Find the entry
        let entries = list_trusted().unwrap();
        let canonical_path = temp_dir
            .path()
            .canonicalize()
            .unwrap_or_else(|_| temp_dir.path().to_path_buf());

        let entry = entries
            .iter()
            .find(|e| e.main_worktree_path == canonical_path)
            .expect("Should find trusted entry");

        // Verify main_worktree_path is set correctly
        assert_eq!(entry.main_worktree_path, canonical_path);

        // Verify trusted_at is set
        assert!(!entry.trusted_at.is_empty());

        // Cleanup
        untrust(temp_dir.path(), &hooks).unwrap();
    }

    #[test]
    fn test_is_trusted_hooks_changed() {
        let temp_dir = TempDir::new().unwrap();
        let hooks1 = Hooks {
            pre_add: vec![HookEntry {
                command: "echo 'original'".to_string(),
                description: None,
            }],
            ..Default::default()
        };

        // Trust the original hooks
        trust(temp_dir.path(), &hooks1).unwrap();
        assert!(is_trusted(temp_dir.path(), &hooks1).unwrap());

        // Change the hooks (different command)
        let hooks2 = Hooks {
            pre_add: vec![HookEntry {
                command: "echo 'modified'".to_string(),
                description: None,
            }],
            ..Default::default()
        };

        // Should not be trusted anymore
        assert!(!is_trusted(temp_dir.path(), &hooks2).unwrap());

        // Cleanup
        untrust(temp_dir.path(), &hooks1).unwrap();
    }

    #[test]
    fn test_is_trusted_hooks_removed() {
        let temp_dir = TempDir::new().unwrap();
        let hooks = Hooks {
            pre_add: vec![HookEntry {
                command: "echo 'test'".to_string(),
                description: None,
            }],
            ..Default::default()
        };

        // Trust the hooks
        trust(temp_dir.path(), &hooks).unwrap();
        assert!(is_trusted(temp_dir.path(), &hooks).unwrap());

        // Remove hooks (empty hooks)
        let empty_hooks = Hooks::default();

        // Empty hooks should be implicitly trusted
        assert!(is_trusted(temp_dir.path(), &empty_hooks).unwrap());

        // Cleanup
        untrust(temp_dir.path(), &hooks).unwrap();
    }

    #[test]
    fn test_is_trusted_different_main_worktree_path() {
        use std::fs;

        let temp_dir1 = TempDir::new().unwrap();
        let temp_dir2 = TempDir::new().unwrap();

        let hooks = Hooks {
            pre_add: vec![HookEntry {
                command: "echo 'test'".to_string(),
                description: None,
            }],
            ..Default::default()
        };

        // Trust hooks for temp_dir1
        trust(temp_dir1.path(), &hooks).unwrap();
        assert!(is_trusted(temp_dir1.path(), &hooks).unwrap());

        // Manually create trust file for same hooks hash but with different main_worktree_path
        let hash = compute_hash(temp_dir1.path(), &hooks).unwrap();
        let dir_name = main_worktree_dir_name(
            &temp_dir1
                .path()
                .canonicalize()
                .unwrap_or_else(|_| temp_dir1.path().to_path_buf()),
        );
        let trust_subdir = trust_dir().unwrap().join(&dir_name);
        fs::create_dir_all(&trust_subdir).unwrap();
        let trust_file = trust_subdir.join(format!("{}.yaml", hash));

        // Overwrite with different main_worktree_path
        let fake_entry = TrustEntry {
            main_worktree_path: temp_dir2.path().canonicalize().unwrap(),
            trusted_at: chrono::Utc::now().to_rfc3339(),
        };
        let content = serde_yaml::to_string(&fake_entry).unwrap();
        fs::write(&trust_file, content).unwrap();

        // Should not be trusted for temp_dir1 anymore (main_worktree_path mismatch)
        assert!(!is_trusted(temp_dir1.path(), &hooks).unwrap());

        // Cleanup
        untrust(temp_dir1.path(), &hooks).unwrap();
    }

    #[test]
    fn test_is_trusted_corrupted_trust_file() {
        use std::fs;

        let temp_dir = TempDir::new().unwrap();
        let hooks = Hooks {
            pre_add: vec![HookEntry {
                command: "echo 'test'".to_string(),
                description: None,
            }],
            ..Default::default()
        };

        // Trust the hooks
        trust(temp_dir.path(), &hooks).unwrap();
        assert!(is_trusted(temp_dir.path(), &hooks).unwrap());

        // Corrupt the trust file
        let hash = compute_hash(temp_dir.path(), &hooks).unwrap();
        let canonical_path = temp_dir
            .path()
            .canonicalize()
            .unwrap_or_else(|_| temp_dir.path().to_path_buf());
        let dir_name = main_worktree_dir_name(&canonical_path);
        let trust_file = trust_dir()
            .unwrap()
            .join(&dir_name)
            .join(format!("{}.yaml", hash));
        fs::write(&trust_file, "invalid yaml content {{{").unwrap();

        // Should return an error
        let result = is_trusted(temp_dir.path(), &hooks);
        assert!(result.is_err());

        // Cleanup (remove corrupted file)
        fs::remove_file(&trust_file).ok();
    }
}
