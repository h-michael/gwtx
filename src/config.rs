use crate::error::{Error, Result};

use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// Config file name
pub const CONFIG_FILE_NAME: &str = ".gwtx.yaml";

/// Load config from the repository root. Returns None if config file doesn't exist.
pub(crate) fn load(repo_root: &Path) -> Result<Option<Config>> {
    let config_path = repo_root.join(CONFIG_FILE_NAME);

    if !config_path.exists() {
        return Ok(None);
    }

    let content = fs::read_to_string(&config_path)?;

    // Parse into RawConfig (permissive, all fields optional)
    let raw: RawConfig = serde_yaml::from_str(&content).map_err(|e| Error::ConfigParse {
        message: e.to_string(),
    })?;

    // Convert to Config (validates and transforms)
    Config::try_from(raw).map(Some)
}

// Raw types for permissive YAML parsing. Missing fields get default values
// instead of parse errors, allowing validation to collect all errors at once.

#[derive(Debug, Deserialize, Default, JsonSchema)]
#[serde(deny_unknown_fields)]
#[schemars(
    title = "gwtx configuration",
    description = "Configuration file for gwtx (git worktree extra)"
)]
pub(crate) struct RawConfig {
    #[serde(default, rename = "defaults")]
    defaults: RawDefaults,
    #[serde(default)]
    worktree: RawWorktree,
    #[serde(default)]
    hooks: RawHooks,
    #[serde(default)]
    mkdir: Vec<RawMkdir>,
    #[serde(default)]
    link: Vec<RawLink>,
    #[serde(default)]
    copy: Vec<RawCopy>,
}

#[derive(Debug, Deserialize, Default, JsonSchema)]
#[serde(deny_unknown_fields)]
#[schemars(
    rename = "Defaults",
    title = "Defaults",
    description = "Global defaults for all operations"
)]
struct RawDefaults {
    on_conflict: Option<OnConflict>,
}

#[derive(Debug, Deserialize, Default, JsonSchema)]
#[serde(deny_unknown_fields)]
#[schemars(
    rename = "Worktree",
    title = "Worktree",
    description = "Worktree path and branch template configuration with template variable support"
)]
struct RawWorktree {
    path_template: Option<String>,
    branch_template: Option<String>,
}

/// Hook entry with command and optional description.
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize, JsonSchema)]
#[serde(deny_unknown_fields)]
#[schemars(
    title = "Hook Entry",
    description = "A hook command with optional description"
)]
pub(crate) struct HookEntry {
    pub command: String,
    #[serde(default)]
    pub description: Option<String>,
}

#[derive(Debug, Deserialize, Default, JsonSchema)]
#[serde(deny_unknown_fields)]
#[schemars(
    rename = "Hooks",
    title = "Hooks",
    description = "Hooks that run before/after worktree operations"
)]
struct RawHooks {
    #[serde(default)]
    pre_add: Vec<HookEntry>,
    #[serde(default)]
    post_add: Vec<HookEntry>,
    #[serde(default)]
    pre_remove: Vec<HookEntry>,
    #[serde(default)]
    post_remove: Vec<HookEntry>,
}

#[derive(Debug, Deserialize, Default, JsonSchema)]
#[serde(deny_unknown_fields)]
#[schemars(
    rename = "MkdirEntry",
    title = "Mkdir Entry",
    description = "Directory creation operation"
)]
struct RawMkdir {
    #[serde(default)]
    path: PathBuf,
    description: Option<String>,
}

#[derive(Debug, Deserialize, Default, JsonSchema)]
#[serde(deny_unknown_fields)]
#[schemars(
    rename = "LinkEntry",
    title = "Link Entry",
    description = "Symlink creation operation with glob pattern support"
)]
struct RawLink {
    #[serde(default)]
    source: PathBuf,
    target: Option<PathBuf>,
    on_conflict: Option<OnConflict>,
    description: Option<String>,
    #[serde(default)]
    ignore_tracked: bool,
}

#[derive(Debug, Deserialize, Default, JsonSchema)]
#[serde(deny_unknown_fields)]
#[schemars(
    rename = "CopyEntry",
    title = "Copy Entry",
    description = "File/directory copy operation"
)]
struct RawCopy {
    #[serde(default)]
    source: PathBuf,
    target: Option<PathBuf>,
    on_conflict: Option<OnConflict>,
    description: Option<String>,
}

// Validated types used by the application. Guaranteed valid after TryFrom conversion.

/// Root configuration from .gwtx.yaml.
#[derive(Debug, Default)]
pub(crate) struct Config {
    pub defaults: Defaults,
    pub worktree: Worktree,
    pub hooks: Hooks,
    pub mkdir: Vec<Mkdir>,
    pub link: Vec<Link>,
    pub copy: Vec<Copy>,
}

impl TryFrom<RawConfig> for Config {
    type Error = Error;

    fn try_from(raw: RawConfig) -> Result<Self> {
        let mut errors = Vec::new();
        let mut targets = HashSet::new();

        // No validation needed for worktree.path
        // It can contain variables or not, both are valid

        // Validate and convert mkdir entries
        let mut mkdir = Vec::with_capacity(raw.mkdir.len());
        for (i, raw_mkdir) in raw.mkdir.into_iter().enumerate() {
            let prefix = format!("mkdir[{i}]");

            if raw_mkdir.path.as_os_str().is_empty() {
                errors.push(format!("  - {prefix}: path is required"));
                continue;
            }

            if let Some(err) = validate_path(&raw_mkdir.path) {
                errors.push(format!("  - {prefix}.path: {err}"));
            }

            if !targets.insert(raw_mkdir.path.clone()) {
                errors.push(format!(
                    "  - {prefix}: duplicate target path: {}",
                    raw_mkdir.path.display()
                ));
            }

            mkdir.push(Mkdir {
                path: raw_mkdir.path,
                description: raw_mkdir.description,
            });
        }

        // Validate and convert link entries
        let mut link = Vec::with_capacity(raw.link.len());
        for (i, raw_link) in raw.link.into_iter().enumerate() {
            let prefix = format!("link[{i}]");

            if raw_link.source.as_os_str().is_empty() {
                errors.push(format!("  - {prefix}: source is required"));
                continue;
            }

            if let Some(err) = validate_path(&raw_link.source) {
                errors.push(format!("  - {prefix}.source: {err}"));
            }

            let target = raw_link
                .target
                .clone()
                .unwrap_or_else(|| raw_link.source.clone());

            if let Some(err) = validate_path(&target) {
                errors.push(format!("  - {prefix}.target: {err}"));
            }

            if !targets.insert(target.clone()) {
                errors.push(format!(
                    "  - {prefix}: duplicate target path: {}",
                    target.display()
                ));
            }

            link.push(Link {
                source: raw_link.source,
                target,
                on_conflict: raw_link.on_conflict,
                description: raw_link.description,
                ignore_tracked: raw_link.ignore_tracked,
            });
        }

        // Validate and convert copy entries
        let mut copy = Vec::with_capacity(raw.copy.len());
        for (i, raw_copy) in raw.copy.into_iter().enumerate() {
            let prefix = format!("copy[{i}]");

            if raw_copy.source.as_os_str().is_empty() {
                errors.push(format!("  - {prefix}: source is required"));
                continue;
            }

            if let Some(err) = validate_path(&raw_copy.source) {
                errors.push(format!("  - {prefix}.source: {err}"));
            }

            let target = raw_copy
                .target
                .clone()
                .unwrap_or_else(|| raw_copy.source.clone());

            if let Some(err) = validate_path(&target) {
                errors.push(format!("  - {prefix}.target: {err}"));
            }

            if !targets.insert(target.clone()) {
                errors.push(format!(
                    "  - {prefix}: duplicate target path: {}",
                    target.display()
                ));
            }

            copy.push(Copy {
                source: raw_copy.source,
                target,
                on_conflict: raw_copy.on_conflict,
                description: raw_copy.description,
            });
        }

        if !errors.is_empty() {
            return Err(Error::ConfigValidation {
                message: errors.join("\n"),
            });
        }

        Ok(Config {
            defaults: Defaults {
                on_conflict: raw.defaults.on_conflict,
            },
            worktree: Worktree {
                path_template: raw.worktree.path_template,
                branch_template: raw.worktree.branch_template,
            },
            hooks: Hooks {
                pre_add: raw.hooks.pre_add,
                post_add: raw.hooks.post_add,
                pre_remove: raw.hooks.pre_remove,
                post_remove: raw.hooks.post_remove,
            },
            mkdir,
            link,
            copy,
        })
    }
}

/// Validate a path and return an error message if invalid.
fn validate_path(path: &Path) -> Option<String> {
    // Check for absolute paths (including Unix-style on Windows for consistent validation)
    let is_absolute = path.is_absolute()
        || path
            .to_str()
            .is_some_and(|s| s.starts_with('/') || s.starts_with('\\'));

    if is_absolute {
        return Some(format!(
            "absolute paths are not allowed: {}",
            path.display()
        ));
    }

    for component in path.components() {
        if component == std::path::Component::ParentDir {
            return Some(format!(
                "path traversal (..) is not allowed: {}",
                path.display()
            ));
        }
    }

    None
}

/// Global options.
#[derive(Debug, Default)]
pub(crate) struct Defaults {
    pub on_conflict: Option<OnConflict>,
}

/// Worktree path generation configuration.
#[derive(Debug, Clone, Default)]
pub(crate) struct Worktree {
    pub path_template: Option<String>,
    pub branch_template: Option<String>,
}

impl Worktree {
    /// Generate suggested worktree path based on configuration.
    /// Returns None if no worktree config is set.
    pub fn generate_path(&self, branch: &str, repository: &str) -> Option<String> {
        self.path_template.as_ref().map(|path_template| {
            let expanded = expand_variables(path_template, branch, repository);
            // If no variables were used, append branch at the end (backward compatibility)
            if expanded == *path_template {
                format!("{}{}", path_template, branch)
            } else {
                expanded
            }
        })
    }

    /// Generate suggested branch name based on branch_template configuration.
    /// Returns None if no branch_template is configured.
    pub fn generate_branch_name(&self, env: &BranchTemplateEnv) -> Option<String> {
        self.branch_template
            .as_ref()
            .map(|template| expand_branch_template(template, env))
    }
}

/// Template environment for branch name generation.
pub(crate) struct BranchTemplateEnv {
    pub commitish: String,
    pub repository: String,
}

/// Expand variables in a string template.
/// Supports {{var}} syntax with optional whitespace.
/// Examples: {{branch}}, {{ branch }}, {{  branch  }}
fn expand_variables(template: &str, branch: &str, repository: &str) -> String {
    let mut result = String::with_capacity(template.len());
    let mut chars = template.chars().peekable();

    while let Some(ch) = chars.next() {
        if ch == '{' {
            // Check if it's {{ (double brace)
            if chars.peek() == Some(&'{') {
                chars.next(); // consume second {

                // Collect variable name until closing }}
                let mut var_name = String::new();
                let mut found_close = false;

                while let Some(ch) = chars.next() {
                    if ch == '}' {
                        // For {{var}}, need to find second }
                        if chars.peek() == Some(&'}') {
                            chars.next(); // consume second }
                            found_close = true;
                            break;
                        } else {
                            var_name.push(ch);
                        }
                    } else {
                        var_name.push(ch);
                    }
                }

                if found_close {
                    // Trim whitespace and replace variable
                    let trimmed = var_name.trim();
                    match trimmed {
                        "branch" => result.push_str(branch),
                        "repository" => result.push_str(repository),
                        _ => {
                            // Unknown variable, keep original
                            result.push_str("{{");
                            result.push_str(&var_name);
                            result.push_str("}}");
                        }
                    }
                } else {
                    // Unclosed brace, keep original
                    result.push_str("{{");
                    result.push_str(&var_name);
                }
            } else {
                // Single {, treat as literal
                result.push('{');
            }
        } else {
            result.push(ch);
        }
    }

    result
}

/// Expand variables in a branch template string.
/// Supports:
/// - {{commitish}}: The commit-ish (branch name, tag, commit hash, etc.)
/// - {{repository}}: Repository name
/// - {{strftime(FORMAT)}}: Date formatting
fn expand_branch_template(template: &str, env: &BranchTemplateEnv) -> String {
    let mut result = String::with_capacity(template.len());
    let mut chars = template.chars().peekable();

    while let Some(ch) = chars.next() {
        if ch == '{' {
            // Check if it's {{ (double brace)
            if chars.peek() == Some(&'{') {
                chars.next(); // consume second {

                let mut content = String::new();
                let mut found_close = false;

                while let Some(c) = chars.next() {
                    if c == '}' {
                        // For {{var}}, need to find second }
                        if chars.peek() == Some(&'}') {
                            chars.next(); // consume second }
                            found_close = true;
                            break;
                        } else {
                            content.push(c);
                        }
                    } else {
                        content.push(c);
                    }
                }

                if found_close {
                    let trimmed = content.trim();
                    let expanded = expand_branch_variable(trimmed, env);
                    result.push_str(&expanded);
                } else {
                    // Unclosed {{, keep original
                    result.push_str("{{");
                    result.push_str(&content);
                }
            } else {
                // Single {, treat as literal
                result.push('{');
            }
        } else {
            result.push(ch);
        }
    }

    result
}

fn expand_branch_variable(var: &str, env: &BranchTemplateEnv) -> String {
    match var {
        "commitish" => env.commitish.clone(),
        "repository" => env.repository.clone(),
        _ if var.starts_with("strftime(") && var.ends_with(')') => {
            let format_str = &var[9..var.len() - 1];
            // Try to format with chrono, fallback to literal if format is invalid
            let formatted = chrono::Local::now().format(format_str);
            let mut result = String::new();
            match std::fmt::write(&mut result, format_args!("{}", formatted)) {
                Ok(_) => result,
                Err(_) => {
                    // Invalid format string, return as literal
                    format!("{{{}}}", var)
                }
            }
        }
        _ => {
            format!("{{{}}}", var)
        }
    }
}

/// Hook commands configuration.
#[derive(Debug, Default, Clone)]
pub(crate) struct Hooks {
    pub pre_add: Vec<HookEntry>,
    pub post_add: Vec<HookEntry>,
    pub pre_remove: Vec<HookEntry>,
    pub post_remove: Vec<HookEntry>,
}

impl Hooks {
    /// Check if any hooks are defined.
    pub fn has_hooks(&self) -> bool {
        !self.pre_add.is_empty()
            || !self.post_add.is_empty()
            || !self.pre_remove.is_empty()
            || !self.post_remove.is_empty()
    }
}

/// Directory creation configuration entry.
#[derive(Debug)]
pub(crate) struct Mkdir {
    pub path: PathBuf,
    pub description: Option<String>,
}

/// Symlink configuration entry.
#[derive(Debug, Clone)]
pub(crate) struct Link {
    pub source: PathBuf,
    pub target: PathBuf, // Always resolved (no Option)
    pub on_conflict: Option<OnConflict>,
    pub description: Option<String>,
    pub ignore_tracked: bool,
}

/// File copy configuration entry.
#[derive(Debug)]
pub(crate) struct Copy {
    pub source: PathBuf,
    pub target: PathBuf, // Always resolved (no Option)
    pub on_conflict: Option<OnConflict>,
    pub description: Option<String>,
}

/// Conflict resolution mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, JsonSchema)]
#[schemars(
    title = "Conflict Resolution Mode",
    description = "How to handle file conflicts during operations"
)]
#[serde(rename_all = "lowercase")]
pub(crate) enum OnConflict {
    Abort,
    Skip,
    Overwrite,
    Backup,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_minimal_config() {
        let yaml = r#"
link:
  - source: ".env.local"
        "#;

        let raw: RawConfig = serde_yaml::from_str(yaml).unwrap();
        let config = Config::try_from(raw).unwrap();
        assert_eq!(config.link.len(), 1);
        assert_eq!(config.link[0].source, PathBuf::from(".env.local"));
        assert_eq!(config.link[0].target, PathBuf::from(".env.local"));
    }

    #[test]
    fn test_parse_full_config() {
        let yaml = r#"
defaults:
  on_conflict: skip

mkdir:
  - path: "tmp/cache"
    description: "Create cache dir"

link:
  - source: ".env.local"
  - source: ".secret/creds.json"
    target: "config/creds.json"
    on_conflict: abort
    description: "Link credentials"

copy:
  - source: ".env.example"
    target: ".env"
    on_conflict: backup
        "#;

        let raw: RawConfig = serde_yaml::from_str(yaml).unwrap();
        let config = Config::try_from(raw).unwrap();

        assert_eq!(config.defaults.on_conflict, Some(OnConflict::Skip));

        assert_eq!(config.mkdir.len(), 1);
        assert_eq!(config.mkdir[0].path, PathBuf::from("tmp/cache"));
        assert_eq!(
            config.mkdir[0].description,
            Some("Create cache dir".to_string())
        );

        assert_eq!(config.link.len(), 2);
        assert_eq!(config.link[1].on_conflict, Some(OnConflict::Abort));
        assert_eq!(
            config.link[1].description,
            Some("Link credentials".to_string())
        );

        assert_eq!(config.copy.len(), 1);
        assert_eq!(config.copy[0].on_conflict, Some(OnConflict::Backup));
    }

    #[test]
    fn test_parse_empty_config() {
        let yaml = "";
        let raw: RawConfig = serde_yaml::from_str(yaml).unwrap();
        let config = Config::try_from(raw).unwrap();
        assert!(config.link.is_empty());
        assert!(config.copy.is_empty());
        assert!(config.mkdir.is_empty());
    }

    #[test]
    fn test_parse_invalid_yaml() {
        let yaml = "invalid: yaml: [[[";
        let result: std::result::Result<RawConfig, _> = serde_yaml::from_str(yaml);
        assert!(result.is_err());
    }

    #[test]
    fn test_validate_missing_source() {
        let yaml = r#"
link:
  - target: ".env"
        "#;

        let raw: RawConfig = serde_yaml::from_str(yaml).unwrap();
        let err = Config::try_from(raw).unwrap_err();
        assert!(err.to_string().contains("source is required"));
    }

    #[test]
    fn test_validate_missing_mkdir_path() {
        let yaml = r#"
mkdir:
  - description: "test"
        "#;

        let raw: RawConfig = serde_yaml::from_str(yaml).unwrap();
        let err = Config::try_from(raw).unwrap_err();
        assert!(err.to_string().contains("path is required"));
    }

    #[test]
    fn test_validate_absolute_path() {
        let yaml = r#"
link:
  - source: "/etc/passwd"
        "#;

        let raw: RawConfig = serde_yaml::from_str(yaml).unwrap();
        let err = Config::try_from(raw).unwrap_err();
        assert!(err.to_string().contains("absolute paths are not allowed"));
    }

    #[test]
    fn test_validate_path_traversal() {
        let yaml = r#"
copy:
  - source: "../../../etc/passwd"
    target: "passwd"
        "#;

        let raw: RawConfig = serde_yaml::from_str(yaml).unwrap();
        let err = Config::try_from(raw).unwrap_err();
        assert!(err.to_string().contains("path traversal"));
    }

    #[test]
    fn test_validate_duplicate_targets() {
        let yaml = r#"
link:
  - source: ".env.local"
    target: ".env"
  - source: ".env.prod"
    target: ".env"
        "#;

        let raw: RawConfig = serde_yaml::from_str(yaml).unwrap();
        let err = Config::try_from(raw).unwrap_err();
        assert!(err.to_string().contains("duplicate target path"));
    }

    #[test]
    fn test_validate_collects_multiple_errors() {
        let yaml = r#"
mkdir:
  - description: "no path"

link:
  - source: "/etc/passwd"

copy:
  - source: "../secret"
    target: "secret"
        "#;

        let raw: RawConfig = serde_yaml::from_str(yaml).unwrap();
        let err = Config::try_from(raw).unwrap_err();
        let msg = err.to_string();
        assert!(msg.contains("path is required"));
        assert!(msg.contains("absolute paths are not allowed"));
        assert!(msg.contains("path traversal"));
    }

    #[test]
    fn test_validate_multiple_missing_sources() {
        let yaml = r#"
copy:
  - description: "copy test1"
    target: "test1-copy"
  - description: "copy test2"
    target: "test2-copy"
        "#;

        let raw: RawConfig = serde_yaml::from_str(yaml).unwrap();
        let err = Config::try_from(raw).unwrap_err();
        let msg = err.to_string();
        assert!(msg.contains("copy[0]: source is required"));
        assert!(msg.contains("copy[1]: source is required"));
    }

    #[test]
    fn test_deny_unknown_fields_top_level() {
        let yaml = r#"
invalid_key: "some value"
link:
  - source: ".env"
        "#;

        let result: std::result::Result<RawConfig, _> = serde_yaml::from_str(yaml);
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.to_string().contains("unknown field"));
    }

    #[test]
    fn test_deny_unknown_fields_in_defaults() {
        let yaml = r#"
defaults:
  invalid_option: true
  on_conflict: skip
        "#;

        let result: std::result::Result<RawConfig, _> = serde_yaml::from_str(yaml);
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.to_string().contains("unknown field"));
    }

    #[test]
    fn test_deny_unknown_fields_in_link() {
        let yaml = r#"
link:
  - source: ".env"
    invalid_field: "test"
        "#;

        let result: std::result::Result<RawConfig, _> = serde_yaml::from_str(yaml);
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.to_string().contains("unknown field"));
    }

    #[test]
    fn test_deny_unknown_fields_in_copy() {
        let yaml = r#"
copy:
  - source: ".env"
    unknown_key: 123
        "#;

        let result: std::result::Result<RawConfig, _> = serde_yaml::from_str(yaml);
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.to_string().contains("unknown field"));
    }

    #[test]
    fn test_deny_unknown_fields_in_mkdir() {
        let yaml = r#"
mkdir:
  - path: "tmp"
    invalid: "value"
        "#;

        let result: std::result::Result<RawConfig, _> = serde_yaml::from_str(yaml);
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.to_string().contains("unknown field"));
    }

    #[test]
    fn test_deny_unknown_fields_in_hooks() {
        let yaml = r#"
hooks:
  invalid_hook: "test"
  pre_add:
    - command: "echo test"
        "#;

        let result: std::result::Result<RawConfig, _> = serde_yaml::from_str(yaml);
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.to_string().contains("unknown field"));
    }

    #[test]
    fn test_deny_unknown_fields_in_hook_entry() {
        let yaml = r#"
hooks:
  pre_add:
    - command: "echo test"
      invalid_field: "value"
        "#;

        let result: std::result::Result<RawConfig, _> = serde_yaml::from_str(yaml);
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.to_string().contains("unknown field"));
    }

    #[test]
    fn test_deny_unknown_fields_in_worktree() {
        let yaml = r#"
worktree:
  path: "../worktrees/"
  invalid_setting: true
        "#;

        let result: std::result::Result<RawConfig, _> = serde_yaml::from_str(yaml);
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.to_string().contains("unknown field"));
    }

    #[test]
    fn test_hooks_has_hooks_empty() {
        let hooks = Hooks::default();
        assert!(!hooks.has_hooks());
    }

    #[test]
    fn test_hooks_has_hooks_with_pre_add() {
        let hooks = Hooks {
            pre_add: vec![HookEntry {
                command: "echo test".to_string(),
                description: None,
            }],
            ..Default::default()
        };
        assert!(hooks.has_hooks());
    }

    #[test]
    fn test_hooks_has_hooks_with_post_add() {
        let hooks = Hooks {
            post_add: vec![HookEntry {
                command: "npm install".to_string(),
                description: None,
            }],
            ..Default::default()
        };
        assert!(hooks.has_hooks());
    }

    #[test]
    fn test_hooks_has_hooks_with_pre_remove() {
        let hooks = Hooks {
            pre_remove: vec![HookEntry {
                command: "echo cleanup".to_string(),
                description: None,
            }],
            ..Default::default()
        };
        assert!(hooks.has_hooks());
    }

    #[test]
    fn test_hooks_has_hooks_with_post_remove() {
        let hooks = Hooks {
            post_remove: vec![HookEntry {
                command: "./scripts/cleanup.sh".to_string(),
                description: None,
            }],
            ..Default::default()
        };
        assert!(hooks.has_hooks());
    }

    #[test]
    fn test_parse_config_with_hooks() {
        let yaml = r#"
hooks:
  pre_add:
    - command: "echo 'pre add'"
  post_add:
    - command: "npm install"
    - command: "mise install"
      description: "Install mise tools"
  pre_remove:
    - command: "echo 'pre remove'"
  post_remove:
    - command: "./scripts/cleanup.sh"
        "#;

        let raw: RawConfig = serde_yaml::from_str(yaml).unwrap();
        let config = Config::try_from(raw).unwrap();

        assert_eq!(config.hooks.pre_add.len(), 1);
        assert_eq!(config.hooks.pre_add[0].command, "echo 'pre add'");
        assert_eq!(config.hooks.pre_add[0].description, None);

        assert_eq!(config.hooks.post_add.len(), 2);
        assert_eq!(config.hooks.post_add[0].command, "npm install");
        assert_eq!(config.hooks.post_add[0].description, None);
        assert_eq!(config.hooks.post_add[1].command, "mise install");
        assert_eq!(
            config.hooks.post_add[1].description,
            Some("Install mise tools".to_string())
        );

        assert_eq!(config.hooks.pre_remove.len(), 1);
        assert_eq!(config.hooks.pre_remove[0].command, "echo 'pre remove'");

        assert_eq!(config.hooks.post_remove.len(), 1);
        assert_eq!(config.hooks.post_remove[0].command, "./scripts/cleanup.sh");
    }

    #[test]
    fn test_parse_config_with_empty_hooks() {
        let yaml = r#"
hooks: {}
        "#;

        let raw: RawConfig = serde_yaml::from_str(yaml).unwrap();
        let config = Config::try_from(raw).unwrap();

        assert!(!config.hooks.has_hooks());
        assert!(config.hooks.pre_add.is_empty());
        assert!(config.hooks.post_add.is_empty());
        assert!(config.hooks.pre_remove.is_empty());
        assert!(config.hooks.post_remove.is_empty());
    }

    #[test]
    fn test_parse_config_without_hooks() {
        let yaml = r#"
mkdir:
  - path: "build"
        "#;

        let raw: RawConfig = serde_yaml::from_str(yaml).unwrap();
        let config = Config::try_from(raw).unwrap();

        assert!(!config.hooks.has_hooks());
    }

    #[test]
    fn test_parse_worktree_path() {
        let yaml = r#"
worktree:
  path_template: "../worktrees/"
        "#;
        let raw: RawConfig = serde_yaml::from_str(yaml).unwrap();
        let config = Config::try_from(raw).unwrap();
        assert_eq!(
            config.worktree.path_template,
            Some("../worktrees/".to_string())
        );
    }

    #[test]
    fn test_parse_worktree_path_with_variables() {
        let yaml = r#"
worktree:
  path_template: "../{repository}-{branch}"
        "#;
        let raw: RawConfig = serde_yaml::from_str(yaml).unwrap();
        let config = Config::try_from(raw).unwrap();
        assert_eq!(
            config.worktree.path_template,
            Some("../{repository}-{branch}".to_string())
        );
    }

    #[test]
    fn test_parse_worktree_empty() {
        let yaml = r#"
worktree: {}
        "#;
        let raw: RawConfig = serde_yaml::from_str(yaml).unwrap();
        let config = Config::try_from(raw).unwrap();
        assert!(config.worktree.path_template.is_none());
    }

    #[test]
    fn test_parse_config_without_worktree() {
        let yaml = r#"
mkdir:
  - path: "build"
        "#;
        let raw: RawConfig = serde_yaml::from_str(yaml).unwrap();
        let config = Config::try_from(raw).unwrap();
        assert!(config.worktree.path_template.is_none());
    }

    #[test]
    fn test_worktree_allows_absolute_path() {
        let yaml = r#"
worktree:
  path_template: "/home/user/worktrees/"
        "#;
        let raw: RawConfig = serde_yaml::from_str(yaml).unwrap();
        let config = Config::try_from(raw).unwrap();
        assert_eq!(
            config.worktree.path_template,
            Some("/home/user/worktrees/".to_string())
        );
    }

    #[test]
    fn test_generate_path_without_variables() {
        let worktree = Worktree {
            path_template: Some("../worktrees/".to_string()),
            branch_template: None,
        };
        let result = worktree.generate_path("feature/foo", "myrepo");
        assert_eq!(result, Some("../worktrees/feature/foo".to_string()));
    }

    #[test]
    fn test_generate_path_with_variables() {
        let worktree = Worktree {
            path_template: Some("../{{repository}}-{{branch}}".to_string()),
            branch_template: None,
        };
        let result = worktree.generate_path("feature/foo", "myrepo");
        assert_eq!(result, Some("../myrepo-feature/foo".to_string()));
    }

    #[test]
    fn test_generate_path_with_branch_only() {
        let worktree = Worktree {
            path_template: Some("../wt-{{branch}}".to_string()),
            branch_template: None,
        };
        let result = worktree.generate_path("main", "myrepo");
        assert_eq!(result, Some("../wt-main".to_string()));
    }

    #[test]
    fn test_generate_path_with_repository_only() {
        let worktree = Worktree {
            path_template: Some("../{{repository}}-worktree".to_string()),
            branch_template: None,
        };
        let result = worktree.generate_path("feature/foo", "myrepo");
        assert_eq!(result, Some("../myrepo-worktree".to_string()));
    }

    #[test]
    fn test_generate_path_no_config() {
        let worktree = Worktree {
            path_template: None,
            branch_template: None,
        };
        let result = worktree.generate_path("feature/foo", "myrepo");
        assert_eq!(result, None);
    }

    #[test]
    fn test_generate_path_branch_with_slashes() {
        let worktree = Worktree {
            path_template: Some("../".to_string()),
            branch_template: None,
        };
        let result = worktree.generate_path("feature/deep/nested", "myrepo");
        assert_eq!(result, Some("../feature/deep/nested".to_string()));
    }

    #[test]
    fn test_generate_path_double_braces() {
        let worktree = Worktree {
            path_template: Some("../{{repository}}-{{branch}}-".to_string()),
            branch_template: None,
        };
        let result = worktree.generate_path("test", "myrepo");
        assert_eq!(result, Some("../myrepo-test-".to_string()));
    }

    #[test]
    fn test_parse_worktree_path_double_braces() {
        let yaml = r#"
worktree:
  path_template: "../{{repository}}-{{branch}}"
        "#;
        let raw: RawConfig = serde_yaml::from_str(yaml).unwrap();
        let config = Config::try_from(raw).unwrap();
        assert_eq!(
            config.worktree.path_template,
            Some("../{{repository}}-{{branch}}".to_string())
        );
    }

    #[test]
    fn test_generate_path_with_whitespace_in_variables() {
        let worktree = Worktree {
            path_template: Some("../{{ branch }}-{{ repository }}".to_string()),
            branch_template: None,
        };
        let result = worktree.generate_path("test", "myrepo");
        assert_eq!(result, Some("../test-myrepo".to_string()));
    }

    #[test]
    fn test_generate_path_with_multiple_spaces() {
        let worktree = Worktree {
            path_template: Some("../{{  branch  }}-{{   repository   }}-".to_string()),
            branch_template: None,
        };
        let result = worktree.generate_path("foo", "bar");
        assert_eq!(result, Some("../foo-bar-".to_string()));
    }

    #[test]
    fn test_generate_path_single_braces_literal() {
        let worktree = Worktree {
            path_template: Some("../{branch}/{{ repository }}".to_string()),
            branch_template: None,
        };
        let result = worktree.generate_path("feature", "myrepo");
        // Single braces should be treated as literal
        assert_eq!(result, Some("../{branch}/myrepo".to_string()));
    }

    #[test]
    fn test_parse_worktree_path_with_spaces() {
        let yaml = r#"
worktree:
  path_template: "../{{ branch }}/{{ repository }}"
        "#;
        let raw: RawConfig = serde_yaml::from_str(yaml).unwrap();
        let config = Config::try_from(raw).unwrap();
        assert!(config.worktree.path_template.is_some());
    }

    #[test]
    fn test_expand_branch_template_commitish() {
        let env = BranchTemplateEnv {
            commitish: "feature/auth".to_string(),
            repository: "myrepo".to_string(),
        };
        let result = expand_branch_template("review/{{commitish}}", &env);
        assert_eq!(result, "review/feature/auth");
    }

    #[test]
    fn test_expand_branch_template_repository() {
        let env = BranchTemplateEnv {
            commitish: "main".to_string(),
            repository: "myrepo".to_string(),
        };
        let result = expand_branch_template("{{repository}}/review/{{commitish}}", &env);
        assert_eq!(result, "myrepo/review/main");
    }

    #[test]
    fn test_expand_branch_template_strftime() {
        let env = BranchTemplateEnv {
            commitish: "fix".to_string(),
            repository: "myrepo".to_string(),
        };
        let result = expand_branch_template("hotfix/{{strftime(%Y)}}/{{commitish}}", &env);
        assert!(result.starts_with("hotfix/20"));
        assert!(result.ends_with("/fix"));
    }

    #[test]
    fn test_expand_branch_template_single_braces_literal() {
        let env = BranchTemplateEnv {
            commitish: "feature".to_string(),
            repository: "myrepo".to_string(),
        };
        // Single braces should be treated as literal
        let result = expand_branch_template("review/{commitish}", &env);
        assert_eq!(result, "review/{commitish}");
    }

    #[test]
    fn test_expand_branch_template_with_spaces() {
        let env = BranchTemplateEnv {
            commitish: "feature".to_string(),
            repository: "myrepo".to_string(),
        };
        let result = expand_branch_template("review/{{ commitish }}", &env);
        assert_eq!(result, "review/feature");
    }

    #[test]
    fn test_expand_branch_template_unknown_variable() {
        let env = BranchTemplateEnv {
            commitish: "main".to_string(),
            repository: "myrepo".to_string(),
        };
        let result = expand_branch_template("{{unknown}}/{{commitish}}", &env);
        assert_eq!(result, "{unknown}/main");
    }

    #[test]
    fn test_generate_branch_name() {
        let worktree = Worktree {
            path_template: None,
            branch_template: Some("review/{{commitish}}".to_string()),
        };
        let env = BranchTemplateEnv {
            commitish: "feature/auth".to_string(),
            repository: "myrepo".to_string(),
        };
        let result = worktree.generate_branch_name(&env);
        assert_eq!(result, Some("review/feature/auth".to_string()));
    }

    #[test]
    fn test_generate_branch_name_none() {
        let worktree = Worktree {
            path_template: None,
            branch_template: None,
        };
        let env = BranchTemplateEnv {
            commitish: "main".to_string(),
            repository: "myrepo".to_string(),
        };
        let result = worktree.generate_branch_name(&env);
        assert_eq!(result, None);
    }

    #[test]
    fn test_parse_worktree_branch_template() {
        let yaml = r#"
worktree:
  path_template: ../worktrees/{{branch}}
  branch_template: review/{{commitish}}
        "#;
        let raw: RawConfig = serde_yaml::from_str(yaml).unwrap();
        let config = Config::try_from(raw).unwrap();
        assert_eq!(
            config.worktree.branch_template,
            Some("review/{{commitish}}".to_string())
        );
    }

    #[test]
    fn test_expand_branch_template_invalid_strftime() {
        let env = BranchTemplateEnv {
            commitish: "main".to_string(),
            repository: "myrepo".to_string(),
        };
        // Invalid format specifiers should not panic, return as literal
        let result = expand_branch_template("feat/{{strftime(%あ)}}", &env);
        assert_eq!(result, "feat/{strftime(%あ)}");

        // Mixed valid and invalid
        let result = expand_branch_template("{{commitish}}-{{strftime(%Y%m%d-%H%M%あ)}}", &env);
        assert!(result.starts_with("main-"));
        // The invalid part should be returned as literal
        assert!(result.contains("strftime"));
    }
}
