use crate::config::Hooks;
use crate::error::{Error, Result};

use std::fs;
use std::path::{Path, PathBuf};

use chrono::Utc;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

const TRUST_DIR_NAME: &str = "gwtx/trusted";

#[derive(Debug, Serialize, Deserialize)]
pub(crate) struct TrustEntry {
    pub repo_root: PathBuf,
    pub trusted_at: String,
    pub hooks: TrustHooks,
}

#[derive(Debug, Serialize, Deserialize)]
pub(crate) struct TrustHooks {
    pub pre_add: Vec<String>,
    pub post_add: Vec<String>,
    pub pre_remove: Vec<String>,
    pub post_remove: Vec<String>,
}

/// Get trust storage directory
/// Uses XDG_DATA_HOME or falls back to ~/.local/share on Linux
fn trust_dir() -> Result<PathBuf> {
    let base = dirs::data_dir().ok_or(Error::TrustStorageNotFound)?;
    Ok(base.join(TRUST_DIR_NAME))
}

/// Compute hash for hooks configuration
pub(crate) fn compute_hash(repo_root: &Path, hooks: &Hooks) -> Result<String> {
    let mut hasher = Sha256::new();

    let canonical_root = repo_root
        .canonicalize()
        .unwrap_or_else(|_| repo_root.to_path_buf());

    hasher.update(canonical_root.to_string_lossy().as_bytes());
    hasher.update(b"\n");

    for cmd in &hooks.pre_add {
        hasher.update(format!("pre_add:{}\n", cmd).as_bytes());
    }
    for cmd in &hooks.post_add {
        hasher.update(format!("post_add:{}\n", cmd).as_bytes());
    }
    for cmd in &hooks.pre_remove {
        hasher.update(format!("pre_remove:{}\n", cmd).as_bytes());
    }
    for cmd in &hooks.post_remove {
        hasher.update(format!("post_remove:{}\n", cmd).as_bytes());
    }

    Ok(format!("{:x}", hasher.finalize()))
}

/// Check if hooks are trusted
pub(crate) fn is_trusted(repo_root: &Path, hooks: &Hooks) -> Result<bool> {
    if !hooks.has_hooks() {
        return Ok(true); // No hooks = implicitly trusted
    }

    let hash = compute_hash(repo_root, hooks)?;
    let trust_file = trust_dir()?.join(format!("{}.toml", hash));

    if !trust_file.exists() {
        return Ok(false);
    }

    // Verify stored content matches current hooks
    let content = fs::read_to_string(&trust_file)?;
    let entry: TrustEntry = toml::from_str(&content).map_err(|e| Error::TrustFileCorrupted {
        message: e.to_string(),
    })?;

    // Verify repo_root matches
    let canonical_root = repo_root
        .canonicalize()
        .unwrap_or_else(|_| repo_root.to_path_buf());

    if entry.repo_root != canonical_root {
        return Ok(false);
    }

    Ok(true)
}

/// Trust hooks for a repository
pub(crate) fn trust(repo_root: &Path, hooks: &Hooks) -> Result<()> {
    if !hooks.has_hooks() {
        return Ok(());
    }

    let trust_path = trust_dir()?;
    fs::create_dir_all(&trust_path)?;

    let hash = compute_hash(repo_root, hooks)?;
    let trust_file = trust_path.join(format!("{}.toml", hash));

    let canonical_root = repo_root
        .canonicalize()
        .unwrap_or_else(|_| repo_root.to_path_buf());

    let entry = TrustEntry {
        repo_root: canonical_root,
        trusted_at: Utc::now().to_rfc3339(),
        hooks: TrustHooks {
            pre_add: hooks.pre_add.clone(),
            post_add: hooks.post_add.clone(),
            pre_remove: hooks.pre_remove.clone(),
            post_remove: hooks.post_remove.clone(),
        },
    };

    let content = toml::to_string_pretty(&entry).map_err(|e| Error::TrustFileSerialization {
        message: e.to_string(),
    })?;

    fs::write(&trust_file, content)?;

    Ok(())
}

/// Untrust hooks for a repository
pub(crate) fn untrust(repo_root: &Path, hooks: &Hooks) -> Result<bool> {
    if !hooks.has_hooks() {
        return Ok(false);
    }

    let hash = compute_hash(repo_root, hooks)?;
    let trust_file = trust_dir()?.join(format!("{}.toml", hash));

    if trust_file.exists() {
        fs::remove_file(&trust_file)?;
        Ok(true)
    } else {
        Ok(false)
    }
}

/// List all trusted repositories
pub(crate) fn list_trusted() -> Result<Vec<TrustEntry>> {
    let trust_path = trust_dir()?;

    if !trust_path.exists() {
        return Ok(Vec::new());
    }

    let mut entries = Vec::new();
    for entry in fs::read_dir(&trust_path)? {
        let entry = entry?;
        let path = entry.path();

        if path.extension().is_some_and(|e| e == "toml") {
            if let Ok(content) = fs::read_to_string(&path) {
                if let Ok(trust_entry) = toml::from_str::<TrustEntry>(&content) {
                    entries.push(trust_entry);
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
            pre_add: vec!["echo 'test1'".to_string()],
            ..Default::default()
        };

        let hooks2 = Hooks {
            pre_add: vec!["echo 'test2'".to_string()],
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
            pre_add: vec!["echo 'test'".to_string()],
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
            pre_add: vec!["echo 'test'".to_string()],
            post_add: vec!["npm install".to_string()],
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

        let found = entries.iter().any(|e| e.repo_root == canonical_path);
        assert!(found, "Trusted repository should be in the list");

        // Cleanup
        untrust(temp_dir.path(), &hooks).unwrap();
    }

    #[test]
    fn test_list_trusted_multiple_repos() {
        let temp_dir1 = TempDir::new().unwrap();
        let temp_dir2 = TempDir::new().unwrap();

        let hooks1 = Hooks {
            pre_add: vec!["echo 'repo1'".to_string()],
            ..Default::default()
        };

        let hooks2 = Hooks {
            pre_add: vec!["echo 'repo2'".to_string()],
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

        let found1 = entries.iter().any(|e| e.repo_root == canonical_path1);
        let found2 = entries.iter().any(|e| e.repo_root == canonical_path2);

        assert!(found1, "First trusted repository should be in the list");
        assert!(found2, "Second trusted repository should be in the list");

        // Cleanup
        untrust(temp_dir1.path(), &hooks1).unwrap();
        untrust(temp_dir2.path(), &hooks2).unwrap();
    }

    #[test]
    fn test_trust_entry_contains_hooks() {
        let temp_dir = TempDir::new().unwrap();
        let hooks = Hooks {
            pre_add: vec!["echo 'pre'".to_string()],
            post_add: vec!["npm install".to_string()],
            pre_remove: vec!["echo 'cleanup'".to_string()],
            post_remove: vec!["./scripts/cleanup.sh".to_string()],
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
            .find(|e| e.repo_root == canonical_path)
            .expect("Should find trusted entry");

        // Verify hooks are stored
        assert_eq!(entry.hooks.pre_add, hooks.pre_add);
        assert_eq!(entry.hooks.post_add, hooks.post_add);
        assert_eq!(entry.hooks.pre_remove, hooks.pre_remove);
        assert_eq!(entry.hooks.post_remove, hooks.post_remove);

        // Verify trusted_at is set
        assert!(!entry.trusted_at.is_empty());

        // Cleanup
        untrust(temp_dir.path(), &hooks).unwrap();
    }
}
