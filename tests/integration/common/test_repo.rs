use assert_cmd::Command;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command as StdCommand;
use tempfile::TempDir;

/// Test repository helper for integration tests
pub struct TestRepo {
    temp_dir: TempDir,
    repo_path: PathBuf,
    worktrees: Vec<PathBuf>,
}

impl TestRepo {
    /// Create a new empty git repository
    pub fn new() -> Self {
        let temp_dir = TempDir::new().expect("Failed to create temp directory");
        let repo_path = temp_dir.path().join("repo");
        fs::create_dir_all(&repo_path).expect("Failed to create repo directory");

        // Initialize git repository
        run_git(&repo_path, &["init"]);
        run_git(&repo_path, &["config", "user.email", "test@example.com"]);
        run_git(&repo_path, &["config", "user.name", "Test User"]);

        // Create initial commit
        let readme = repo_path.join("README.md");
        fs::write(&readme, "# Test Repository\n").expect("Failed to write README");
        run_git(&repo_path, &["add", "README.md"]);
        run_git(&repo_path, &["commit", "-m", "Initial commit"]);

        Self {
            temp_dir,
            repo_path,
            worktrees: Vec::new(),
        }
    }

    /// Create a new repository with the given .gwtx/config.yaml config
    pub fn with_config(yaml: &str) -> Self {
        let repo = Self::new();
        repo.write_config(yaml);
        repo
    }

    /// Write .gwtx/config.yaml configuration file
    pub fn write_config(&self, yaml: &str) {
        let gwtx_dir = self.repo_path.join(".gwtx");
        fs::create_dir_all(&gwtx_dir).expect("Failed to create .gwtx directory");
        let config_path = gwtx_dir.join("config.yaml");
        fs::write(&config_path, yaml).expect("Failed to write config");
        run_git(&self.repo_path, &["add", ".gwtx/config.yaml"]);
        run_git(&self.repo_path, &["commit", "-m", "Add gwtx config"]);
    }

    /// Create a file in the repository
    pub fn create_file(&self, path: &str, content: &str) {
        let file_path = self.repo_path.join(path);
        if let Some(parent) = file_path.parent() {
            fs::create_dir_all(parent).expect("Failed to create parent directories");
        }
        fs::write(&file_path, content).expect("Failed to write file");
    }

    /// Create a file and commit it
    pub fn create_file_and_commit(&self, path: &str, content: &str, message: &str) {
        self.create_file(path, content);
        run_git(&self.repo_path, &["add", path]);
        run_git(&self.repo_path, &["commit", "-m", message]);
    }

    /// Get a gwtx Command configured for this repository
    pub fn gwtx(&self) -> Command {
        let mut cmd = Command::cargo_bin("gwtx").expect("Failed to find gwtx binary");
        cmd.current_dir(&self.repo_path);
        // Use isolated trust directory in tests
        if let Ok(trust_dir) = std::env::var("GWTX_TRUST_DIR") {
            cmd.env("GWTX_TRUST_DIR", trust_dir);
        }
        cmd
    }

    /// Trust the configuration (auto-accept with --yes flag)
    pub fn trust_config(&self) {
        self.gwtx().args(["trust", "--yes"]).assert().success();
    }

    /// Get the repository path
    pub fn path(&self) -> &Path {
        &self.repo_path
    }

    /// Get a worktree path (relative to temp directory, not repo)
    pub fn worktree_path(&self, name: &str) -> PathBuf {
        self.temp_dir.path().join(name)
    }

    /// Register a worktree for cleanup
    pub fn register_worktree(&mut self, path: PathBuf) {
        self.worktrees.push(path);
    }

    /// Clear registered worktrees (use after manual removal)
    pub fn clear_registered_worktrees(&mut self) {
        self.worktrees.clear();
    }

    /// Check if a file exists in a worktree
    pub fn worktree_file_exists(&self, worktree_name: &str, file_path: &str) -> bool {
        self.worktree_path(worktree_name).join(file_path).exists()
    }

    /// Check if a symlink exists in a worktree
    pub fn worktree_symlink_exists(&self, worktree_name: &str, file_path: &str) -> bool {
        let path = self.worktree_path(worktree_name).join(file_path);
        path.symlink_metadata()
            .map(|m| m.file_type().is_symlink())
            .unwrap_or(false)
    }

    /// Read file content from a worktree
    pub fn read_worktree_file(&self, worktree_name: &str, file_path: &str) -> String {
        let path = self.worktree_path(worktree_name).join(file_path);
        fs::read_to_string(path).expect("Failed to read file")
    }

    /// Check if a directory exists in a worktree
    pub fn worktree_dir_exists(&self, worktree_name: &str, dir_path: &str) -> bool {
        self.worktree_path(worktree_name).join(dir_path).is_dir()
    }

    /// List worktrees using git command
    pub fn list_worktrees(&self) -> Vec<String> {
        let output = StdCommand::new("git")
            .current_dir(&self.repo_path)
            .args(["worktree", "list", "--porcelain"])
            .output()
            .expect("Failed to run git worktree list");

        String::from_utf8_lossy(&output.stdout)
            .lines()
            .filter_map(|line| line.strip_prefix("worktree "))
            .map(|s| s.to_string())
            .collect()
    }
}

impl Drop for TestRepo {
    fn drop(&mut self) {
        // Clean up worktrees before temp directory is removed
        for worktree in &self.worktrees {
            let _ = StdCommand::new("git")
                .current_dir(&self.repo_path)
                .args(["worktree", "remove", "--force"])
                .arg(worktree)
                .output();
        }
    }
}

fn run_git(dir: &Path, args: &[&str]) {
    let output = StdCommand::new("git")
        .current_dir(dir)
        .args(args)
        .output()
        .unwrap_or_else(|e| panic!("Failed to run git {:?}: {}", args, e));

    if !output.status.success() {
        panic!(
            "git {:?} failed:\nstdout: {}\nstderr: {}",
            args,
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
    }
}
