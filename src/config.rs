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
    #[serde(default)]
    options: RawOptions,
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
    rename = "Options",
    title = "Options",
    description = "Global options for all operations"
)]
struct RawOptions {
    on_conflict: Option<OnConflict>,
}

#[derive(Debug, Deserialize, Default, JsonSchema)]
#[serde(deny_unknown_fields)]
#[schemars(
    rename = "Worktree",
    title = "Worktree",
    description = "Worktree path configuration with template variable support"
)]
struct RawWorktree {
    path: Option<String>,
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
    skip_tracked: bool,
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
    pub options: Options,
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
                skip_tracked: raw_link.skip_tracked,
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
            options: Options {
                on_conflict: raw.options.on_conflict,
            },
            worktree: Worktree {
                path: raw.worktree.path,
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
pub(crate) struct Options {
    pub on_conflict: Option<OnConflict>,
}

/// Worktree path generation configuration.
#[derive(Debug, Clone, Default)]
pub(crate) struct Worktree {
    pub path: Option<String>,
}

impl Worktree {
    /// Generate suggested worktree path based on configuration.
    /// Returns None if no worktree config is set.
    pub fn generate_path(&self, branch: &str, repo_name: &str) -> Option<String> {
        self.path.as_ref().map(|path_template| {
            let expanded = expand_variables(path_template, branch, repo_name);
            // If no variables were used, append branch at the end (backward compatibility)
            if expanded == *path_template {
                format!("{}{}", path_template, branch)
            } else {
                expanded
            }
        })
    }
}

/// Expand variables in a string template.
/// Supports {var} and {{var}} syntax with optional whitespace.
/// Examples: {branch}, {{ branch }}, {{  branch  }}
fn expand_variables(template: &str, branch: &str, repo_name: &str) -> String {
    let mut result = String::with_capacity(template.len());
    let mut chars = template.chars().peekable();

    while let Some(ch) = chars.next() {
        if ch == '{' {
            // Check if it's {{ (double brace)
            let is_double = chars.peek() == Some(&'{');
            if is_double {
                chars.next(); // consume second {
            }

            // Collect variable name until closing brace(s)
            let mut var_name = String::new();
            let mut found_close = false;

            while let Some(ch) = chars.next() {
                if ch == '}' {
                    if is_double {
                        // For {{var}}, need to find second }
                        if chars.peek() == Some(&'}') {
                            chars.next(); // consume second }
                            found_close = true;
                            break;
                        } else {
                            var_name.push(ch);
                        }
                    } else {
                        // For {var}, one } is enough
                        found_close = true;
                        break;
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
                    "repo_name" => result.push_str(repo_name),
                    _ => {
                        // Unknown variable, keep original
                        if is_double {
                            result.push_str("{{");
                            result.push_str(&var_name);
                            result.push_str("}}");
                        } else {
                            result.push('{');
                            result.push_str(&var_name);
                            result.push('}');
                        }
                    }
                }
            } else {
                // Unclosed brace, keep original
                if is_double {
                    result.push_str("{{");
                } else {
                    result.push('{');
                }
                result.push_str(&var_name);
            }
        } else {
            result.push(ch);
        }
    }

    result
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
    pub skip_tracked: bool,
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
options:
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

        assert_eq!(config.options.on_conflict, Some(OnConflict::Skip));

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
    fn test_deny_unknown_fields_in_options() {
        let yaml = r#"
options:
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
  path: "../worktrees/"
        "#;
        let raw: RawConfig = serde_yaml::from_str(yaml).unwrap();
        let config = Config::try_from(raw).unwrap();
        assert_eq!(config.worktree.path, Some("../worktrees/".to_string()));
    }

    #[test]
    fn test_parse_worktree_path_with_variables() {
        let yaml = r#"
worktree:
  path: "../{repo_name}-{branch}"
        "#;
        let raw: RawConfig = serde_yaml::from_str(yaml).unwrap();
        let config = Config::try_from(raw).unwrap();
        assert_eq!(
            config.worktree.path,
            Some("../{repo_name}-{branch}".to_string())
        );
    }

    #[test]
    fn test_parse_worktree_empty() {
        let yaml = r#"
worktree: {}
        "#;
        let raw: RawConfig = serde_yaml::from_str(yaml).unwrap();
        let config = Config::try_from(raw).unwrap();
        assert!(config.worktree.path.is_none());
    }

    #[test]
    fn test_parse_config_without_worktree() {
        let yaml = r#"
mkdir:
  - path: "build"
        "#;
        let raw: RawConfig = serde_yaml::from_str(yaml).unwrap();
        let config = Config::try_from(raw).unwrap();
        assert!(config.worktree.path.is_none());
    }

    #[test]
    fn test_worktree_allows_absolute_path() {
        let yaml = r#"
worktree:
  path: "/home/user/worktrees/"
        "#;
        let raw: RawConfig = serde_yaml::from_str(yaml).unwrap();
        let config = Config::try_from(raw).unwrap();
        assert_eq!(
            config.worktree.path,
            Some("/home/user/worktrees/".to_string())
        );
    }

    #[test]
    fn test_generate_path_without_variables() {
        let worktree = Worktree {
            path: Some("../worktrees/".to_string()),
        };
        let result = worktree.generate_path("feature/foo", "myrepo");
        assert_eq!(result, Some("../worktrees/feature/foo".to_string()));
    }

    #[test]
    fn test_generate_path_with_variables() {
        let worktree = Worktree {
            path: Some("../{repo_name}-{branch}".to_string()),
        };
        let result = worktree.generate_path("feature/foo", "myrepo");
        assert_eq!(result, Some("../myrepo-feature/foo".to_string()));
    }

    #[test]
    fn test_generate_path_with_branch_only() {
        let worktree = Worktree {
            path: Some("../wt-{branch}".to_string()),
        };
        let result = worktree.generate_path("main", "myrepo");
        assert_eq!(result, Some("../wt-main".to_string()));
    }

    #[test]
    fn test_generate_path_with_repo_name_only() {
        let worktree = Worktree {
            path: Some("../{repo_name}-worktree".to_string()),
        };
        let result = worktree.generate_path("feature/foo", "myrepo");
        assert_eq!(result, Some("../myrepo-worktree".to_string()));
    }

    #[test]
    fn test_generate_path_no_config() {
        let worktree = Worktree { path: None };
        let result = worktree.generate_path("feature/foo", "myrepo");
        assert_eq!(result, None);
    }

    #[test]
    fn test_generate_path_branch_with_slashes() {
        let worktree = Worktree {
            path: Some("../".to_string()),
        };
        let result = worktree.generate_path("feature/deep/nested", "myrepo");
        assert_eq!(result, Some("../feature/deep/nested".to_string()));
    }

    #[test]
    fn test_generate_path_double_braces() {
        let worktree = Worktree {
            path: Some("../{{repo_name}}-{{branch}}-".to_string()),
        };
        let result = worktree.generate_path("test", "myrepo");
        assert_eq!(result, Some("../myrepo-test-".to_string()));
    }

    #[test]
    fn test_parse_worktree_path_double_braces() {
        let yaml = r#"
worktree:
  path: "../{{repo_name}}-{{branch}}"
        "#;
        let raw: RawConfig = serde_yaml::from_str(yaml).unwrap();
        let config = Config::try_from(raw).unwrap();
        assert_eq!(
            config.worktree.path,
            Some("../{{repo_name}}-{{branch}}".to_string())
        );
    }

    #[test]
    fn test_generate_path_with_whitespace_in_variables() {
        let worktree = Worktree {
            path: Some("../{{ branch }}-{{ repo_name }}".to_string()),
        };
        let result = worktree.generate_path("test", "myrepo");
        assert_eq!(result, Some("../test-myrepo".to_string()));
    }

    #[test]
    fn test_generate_path_with_multiple_spaces() {
        let worktree = Worktree {
            path: Some("../{{  branch  }}-{{   repo_name   }}-".to_string()),
        };
        let result = worktree.generate_path("foo", "bar");
        assert_eq!(result, Some("../foo-bar-".to_string()));
    }

    #[test]
    fn test_generate_path_mixed_formats() {
        let worktree = Worktree {
            path: Some("../{branch}/{{ repo_name }}".to_string()),
        };
        let result = worktree.generate_path("feature", "myrepo");
        assert_eq!(result, Some("../feature/myrepo".to_string()));
    }

    #[test]
    fn test_parse_worktree_path_with_spaces() {
        let yaml = r#"
worktree:
  path: "../{{ branch }}/{{ repo_name }}"
        "#;
        let raw: RawConfig = serde_yaml::from_str(yaml).unwrap();
        let config = Config::try_from(raw).unwrap();
        assert!(config.worktree.path.is_some());
    }
}
