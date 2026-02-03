use assert_cmd::Command;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command as StdCommand;
use tempfile::TempDir;

/// Check if jj is available on the system
pub fn jj_available() -> bool {
    StdCommand::new("jj")
        .arg("--version")
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

/// Test repository helper for jj integration tests
pub struct JjTestRepo {
    temp_dir: TempDir,
    repo_path: PathBuf,
    workspaces: Vec<PathBuf>,
}

impl JjTestRepo {
    /// Create a new jj repository (non-colocated)
    pub fn new() -> Self {
        let temp_dir = TempDir::new().expect("Failed to create temp directory");
        let repo_path = temp_dir.path().join("repo");
        fs::create_dir_all(&repo_path).expect("Failed to create repo directory");

        // Initialize jj repository
        run_jj(&repo_path, &["git", "init"]);

        // Create initial file
        let readme = repo_path.join("README.md");
        fs::write(&readme, "# Test Repository\n").expect("Failed to write README");

        // Describe the initial change
        run_jj(&repo_path, &["describe", "-m", "Initial commit"]);
        // Create a new change to work on
        run_jj(&repo_path, &["new"]);

        Self {
            temp_dir,
            repo_path,
            workspaces: Vec::new(),
        }
    }

    /// Create a colocated jj repository (with .git)
    pub fn new_colocated() -> Self {
        let temp_dir = TempDir::new().expect("Failed to create temp directory");
        let repo_path = temp_dir.path().join("repo");
        fs::create_dir_all(&repo_path).expect("Failed to create repo directory");

        // Initialize colocated jj repository
        run_jj(&repo_path, &["git", "init", "--colocate"]);

        // Create initial file
        let readme = repo_path.join("README.md");
        fs::write(&readme, "# Test Repository\n").expect("Failed to write README");

        // Describe the initial change
        run_jj(&repo_path, &["describe", "-m", "Initial commit"]);
        // Create a new change to work on
        run_jj(&repo_path, &["new"]);

        Self {
            temp_dir,
            repo_path,
            workspaces: Vec::new(),
        }
    }

    /// Create a new repository with the given .kabu/config.yaml config
    pub fn with_config(yaml: &str) -> Self {
        let repo = Self::new();
        repo.write_config(yaml);
        repo
    }

    /// Create a colocated repository with the given .kabu/config.yaml config
    pub fn with_config_colocated(yaml: &str) -> Self {
        let repo = Self::new_colocated();
        repo.write_config(yaml);
        repo
    }

    /// Write .kabu/config.yaml configuration file
    pub fn write_config(&self, yaml: &str) {
        let kabu_dir = self.repo_path.join(".kabu");
        fs::create_dir_all(&kabu_dir).expect("Failed to create .kabu directory");
        let config_path = kabu_dir.join("config.yaml");
        fs::write(&config_path, yaml).expect("Failed to write config");
        run_jj(&self.repo_path, &["describe", "-m", "Add kabu config"]);
        run_jj(&self.repo_path, &["new"]);
    }

    /// Create a file in the repository
    pub fn create_file(&self, path: &str, content: &str) {
        let file_path = self.repo_path.join(path);
        if let Some(parent) = file_path.parent() {
            fs::create_dir_all(parent).expect("Failed to create parent directories");
        }
        fs::write(&file_path, content).expect("Failed to write file");
    }

    /// Create a file and snapshot it
    pub fn create_file_and_snapshot(&self, path: &str, content: &str, message: &str) {
        self.create_file(path, content);
        run_jj(&self.repo_path, &["describe", "-m", message]);
        run_jj(&self.repo_path, &["new"]);
    }

    /// Get a kabu Command configured for this repository
    pub fn kabu(&self) -> Command {
        let mut cmd = Command::cargo_bin("kabu").expect("Failed to find kabu binary");
        cmd.current_dir(&self.repo_path);
        // Use isolated trust directory in tests
        if let Ok(trust_dir) = std::env::var("KABU_TRUST_DIR") {
            cmd.env("KABU_TRUST_DIR", trust_dir);
        }
        cmd
    }

    /// Trust the configuration (auto-accept with --yes flag)
    pub fn trust_config(&self) {
        self.kabu().args(["trust", "--yes"]).assert().success();
    }

    /// Get the repository path
    pub fn path(&self) -> &Path {
        &self.repo_path
    }

    /// Get a workspace path (inside the repo directory)
    pub fn workspace_path(&self, name: &str) -> PathBuf {
        self.repo_path.join(name)
    }

    /// Register a workspace for cleanup
    pub fn register_workspace(&mut self, path: PathBuf) {
        self.workspaces.push(path);
    }

    /// Clear registered workspaces (use after manual removal)
    pub fn clear_registered_workspaces(&mut self) {
        self.workspaces.clear();
    }

    /// Check if a file exists in a workspace
    pub fn workspace_file_exists(&self, workspace_name: &str, file_path: &str) -> bool {
        self.workspace_path(workspace_name).join(file_path).exists()
    }

    /// Check if a symlink exists in a workspace
    pub fn workspace_symlink_exists(&self, workspace_name: &str, file_path: &str) -> bool {
        let path = self.workspace_path(workspace_name).join(file_path);
        path.symlink_metadata()
            .map(|m| m.file_type().is_symlink())
            .unwrap_or(false)
    }

    /// Read file content from a workspace
    pub fn read_workspace_file(&self, workspace_name: &str, file_path: &str) -> String {
        let path = self.workspace_path(workspace_name).join(file_path);
        fs::read_to_string(path).expect("Failed to read file")
    }

    /// Check if a directory exists in a workspace
    pub fn workspace_dir_exists(&self, workspace_name: &str, dir_path: &str) -> bool {
        self.workspace_path(workspace_name).join(dir_path).is_dir()
    }

    /// List workspaces using jj command
    pub fn list_workspaces(&self) -> Vec<String> {
        let output = StdCommand::new("jj")
            .current_dir(&self.repo_path)
            .args(["workspace", "list"])
            .output()
            .expect("Failed to run jj workspace list");

        String::from_utf8_lossy(&output.stdout)
            .lines()
            .filter_map(|line| {
                // Parse workspace list output format: "name: path"
                line.split(':').next().map(|s| s.trim().to_string())
            })
            .collect()
    }
}

impl Drop for JjTestRepo {
    fn drop(&mut self) {
        // Clean up workspaces before temp directory is removed
        for workspace in &self.workspaces {
            // Get workspace name from path
            if let Some(name) = workspace.file_name().and_then(|n| n.to_str()) {
                let _ = StdCommand::new("jj")
                    .current_dir(&self.repo_path)
                    .args(["workspace", "forget", name])
                    .output();
            }
            // Remove the directory
            let _ = fs::remove_dir_all(workspace);
        }
    }
}

fn run_jj(dir: &Path, args: &[&str]) {
    let output = StdCommand::new("jj")
        .current_dir(dir)
        .args(args)
        .output()
        .unwrap_or_else(|e| panic!("Failed to run jj {:?}: {}", args, e));

    if !output.status.success() {
        panic!(
            "jj {:?} failed:\nstdout: {}\nstderr: {}",
            args,
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
    }
}
