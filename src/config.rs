use crate::error::{Error, Result};

use std::fs;
use std::path::{Path, PathBuf};

use serde::Deserialize;

/// Config file name
pub const CONFIG_FILE_NAME: &str = ".gwtx.toml";

/// Load config from the repository root
pub(crate) fn load(repo_root: &Path) -> Result<Config> {
    let config_path = repo_root.join(CONFIG_FILE_NAME);

    if !config_path.exists() {
        return Err(Error::ConfigNotFound { path: config_path });
    }

    let content = fs::read_to_string(&config_path)?;

    toml::from_str(&content).map_err(|e| Error::ConfigParse {
        message: e.to_string(),
    })
}

/// Root configuration from .gwtx.toml.
#[derive(Debug, Deserialize, Default)]
pub(crate) struct Config {
    #[serde(default)]
    pub options: Options,

    #[serde(default)]
    pub mkdir: Vec<Mkdir>,

    #[serde(default)]
    pub link: Vec<Link>,

    #[serde(default)]
    pub copy: Vec<Copy>,
}

/// Global options.
#[derive(Debug, Deserialize, Default)]
pub(crate) struct Options {
    pub on_conflict: Option<OnConflict>,
}

/// Directory creation configuration entry.
#[derive(Debug, Deserialize)]
pub(crate) struct Mkdir {
    pub path: PathBuf,
    pub description: Option<String>,
}

/// Symlink configuration entry.
#[derive(Debug, Deserialize)]
pub(crate) struct Link {
    pub source: PathBuf,
    pub target: Option<PathBuf>,
    pub on_conflict: Option<OnConflict>,
    pub description: Option<String>,
}

impl Link {
    /// Target path, defaults to source if not specified.
    pub fn target(&self) -> &PathBuf {
        self.target.as_ref().unwrap_or(&self.source)
    }
}

/// File copy configuration entry.
#[derive(Debug, Deserialize)]
pub(crate) struct Copy {
    pub source: PathBuf,
    pub target: Option<PathBuf>,
    pub on_conflict: Option<OnConflict>,
    pub description: Option<String>,
}

impl Copy {
    /// Target path, defaults to source if not specified.
    pub fn target(&self) -> &PathBuf {
        self.target.as_ref().unwrap_or(&self.source)
    }
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
    fn test_link_target_defaults_to_source() {
        let link = Link {
            source: PathBuf::from(".env.local"),
            target: None,
            on_conflict: None,
            description: None,
        };
        assert_eq!(link.target(), &PathBuf::from(".env.local"));
    }

    #[test]
    fn test_link_target_uses_explicit_value() {
        let link = Link {
            source: PathBuf::from(".env.local"),
            target: Some(PathBuf::from("config/.env")),
            on_conflict: None,
            description: None,
        };
        assert_eq!(link.target(), &PathBuf::from("config/.env"));
    }

    #[test]
    fn test_copy_target_defaults_to_source() {
        let copy = Copy {
            source: PathBuf::from(".env.example"),
            target: None,
            on_conflict: None,
            description: None,
        };
        assert_eq!(copy.target(), &PathBuf::from(".env.example"));
    }

    #[test]
    fn test_parse_minimal_config() {
        let toml = r#"
            [[link]]
            source = ".env.local"
        "#;

        let config: Config = toml::from_str(toml).unwrap();
        assert_eq!(config.link.len(), 1);
        assert_eq!(config.link[0].source, PathBuf::from(".env.local"));
        assert!(config.link[0].target.is_none());
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

        let config: Config = toml::from_str(toml).unwrap();

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
        let config: Config = toml::from_str(toml).unwrap();
        assert!(config.link.is_empty());
        assert!(config.copy.is_empty());
        assert!(config.mkdir.is_empty());
    }

    #[test]
    fn test_parse_invalid_toml() {
        let toml = "invalid toml [[[";
        let result: std::result::Result<Config, _> = toml::from_str(toml);
        assert!(result.is_err());
    }
}
