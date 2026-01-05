use crate::error::{Error, Result};

use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};

use serde::Deserialize;

/// Config file name
pub const CONFIG_FILE_NAME: &str = ".gwtx.toml";

/// Load config from the repository root. Returns None if config file doesn't exist.
pub(crate) fn load(repo_root: &Path) -> Result<Option<Config>> {
    let config_path = repo_root.join(CONFIG_FILE_NAME);

    if !config_path.exists() {
        return Ok(None);
    }

    let content = fs::read_to_string(&config_path)?;

    // Parse into RawConfig (permissive, all fields optional)
    let raw: RawConfig = toml::from_str(&content).map_err(|e| Error::ConfigParse {
        message: e.message().to_string(),
    })?;

    // Convert to Config (validates and transforms)
    Config::try_from(raw).map(Some)
}

// Raw types for permissive TOML parsing. Missing fields get default values
// instead of parse errors, allowing validation to collect all errors at once.

#[derive(Debug, Deserialize, Default)]
struct RawConfig {
    #[serde(default)]
    options: RawOptions,
    #[serde(default)]
    mkdir: Vec<RawMkdir>,
    #[serde(default)]
    link: Vec<RawLink>,
    #[serde(default)]
    copy: Vec<RawCopy>,
}

#[derive(Debug, Deserialize, Default)]
struct RawOptions {
    on_conflict: Option<OnConflict>,
}

#[derive(Debug, Deserialize, Default)]
struct RawMkdir {
    #[serde(default)]
    path: PathBuf,
    description: Option<String>,
}

#[derive(Debug, Deserialize, Default)]
struct RawLink {
    #[serde(default)]
    source: PathBuf,
    target: Option<PathBuf>,
    on_conflict: Option<OnConflict>,
    description: Option<String>,
}

#[derive(Debug, Deserialize, Default)]
struct RawCopy {
    #[serde(default)]
    source: PathBuf,
    target: Option<PathBuf>,
    on_conflict: Option<OnConflict>,
    description: Option<String>,
}

// Validated types used by the application. Guaranteed valid after TryFrom conversion.

/// Root configuration from .gwtx.toml.
#[derive(Debug, Default)]
pub(crate) struct Config {
    pub options: Options,
    pub mkdir: Vec<Mkdir>,
    pub link: Vec<Link>,
    pub copy: Vec<Copy>,
}

impl TryFrom<RawConfig> for Config {
    type Error = Error;

    fn try_from(raw: RawConfig) -> Result<Self> {
        let mut errors = Vec::new();
        let mut targets = HashSet::new();

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
            mkdir,
            link,
            copy,
        })
    }
}

/// Validate a path and return an error message if invalid.
fn validate_path(path: &Path) -> Option<String> {
    if path.is_absolute() {
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

/// Directory creation configuration entry.
#[derive(Debug)]
pub(crate) struct Mkdir {
    pub path: PathBuf,
    pub description: Option<String>,
}

/// Symlink configuration entry.
#[derive(Debug)]
pub(crate) struct Link {
    pub source: PathBuf,
    pub target: PathBuf, // Always resolved (no Option)
    pub on_conflict: Option<OnConflict>,
    pub description: Option<String>,
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
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
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
        let toml = r#"
            [[link]]
            source = ".env.local"
        "#;

        let raw: RawConfig = toml::from_str(toml).unwrap();
        let config = Config::try_from(raw).unwrap();
        assert_eq!(config.link.len(), 1);
        assert_eq!(config.link[0].source, PathBuf::from(".env.local"));
        assert_eq!(config.link[0].target, PathBuf::from(".env.local"));
    }

    #[test]
    fn test_parse_full_config() {
        let toml = r#"
            [options]
            on_conflict = "skip"

            [[mkdir]]
            path = "tmp/cache"
            description = "Create cache dir"

            [[link]]
            source = ".env.local"

            [[link]]
            source = ".secret/creds.json"
            target = "config/creds.json"
            on_conflict = "abort"
            description = "Link credentials"

            [[copy]]
            source = ".env.example"
            target = ".env"
            on_conflict = "backup"
        "#;

        let raw: RawConfig = toml::from_str(toml).unwrap();
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
        let toml = "";
        let raw: RawConfig = toml::from_str(toml).unwrap();
        let config = Config::try_from(raw).unwrap();
        assert!(config.link.is_empty());
        assert!(config.copy.is_empty());
        assert!(config.mkdir.is_empty());
    }

    #[test]
    fn test_parse_invalid_toml() {
        let toml = "invalid toml [[[";
        let result: std::result::Result<RawConfig, _> = toml::from_str(toml);
        assert!(result.is_err());
    }

    #[test]
    fn test_validate_missing_source() {
        let toml = r#"
            [[link]]
            target = ".env"
        "#;

        let raw: RawConfig = toml::from_str(toml).unwrap();
        let err = Config::try_from(raw).unwrap_err();
        assert!(err.to_string().contains("source is required"));
    }

    #[test]
    fn test_validate_missing_mkdir_path() {
        let toml = r#"
            [[mkdir]]
            description = "test"
        "#;

        let raw: RawConfig = toml::from_str(toml).unwrap();
        let err = Config::try_from(raw).unwrap_err();
        assert!(err.to_string().contains("path is required"));
    }

    #[test]
    fn test_validate_absolute_path() {
        let toml = r#"
            [[link]]
            source = "/etc/passwd"
        "#;

        let raw: RawConfig = toml::from_str(toml).unwrap();
        let err = Config::try_from(raw).unwrap_err();
        assert!(err.to_string().contains("absolute paths are not allowed"));
    }

    #[test]
    fn test_validate_path_traversal() {
        let toml = r#"
            [[copy]]
            source = "../../../etc/passwd"
            target = "passwd"
        "#;

        let raw: RawConfig = toml::from_str(toml).unwrap();
        let err = Config::try_from(raw).unwrap_err();
        assert!(err.to_string().contains("path traversal"));
    }

    #[test]
    fn test_validate_duplicate_targets() {
        let toml = r#"
            [[link]]
            source = ".env.local"
            target = ".env"

            [[link]]
            source = ".env.prod"
            target = ".env"
        "#;

        let raw: RawConfig = toml::from_str(toml).unwrap();
        let err = Config::try_from(raw).unwrap_err();
        assert!(err.to_string().contains("duplicate target path"));
    }

    #[test]
    fn test_validate_collects_multiple_errors() {
        let toml = r#"
            [[mkdir]]
            description = "no path"

            [[link]]
            source = "/etc/passwd"

            [[copy]]
            source = "../secret"
            target = "secret"
        "#;

        let raw: RawConfig = toml::from_str(toml).unwrap();
        let err = Config::try_from(raw).unwrap_err();
        let msg = err.to_string();
        assert!(msg.contains("path is required"));
        assert!(msg.contains("absolute paths are not allowed"));
        assert!(msg.contains("path traversal"));
    }

    #[test]
    fn test_validate_multiple_missing_sources() {
        let toml = r#"
            [[copy]]
            description = "copy test1"
            target = "test1-copy"

            [[copy]]
            description = "copy test2"
            target = "test2-copy"
        "#;

        let raw: RawConfig = toml::from_str(toml).unwrap();
        let err = Config::try_from(raw).unwrap_err();
        let msg = err.to_string();
        assert!(msg.contains("copy[0]: source is required"));
        assert!(msg.contains("copy[1]: source is required"));
    }
}
