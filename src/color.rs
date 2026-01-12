use std::io::IsTerminal;

use owo_colors::OwoColorize;

/// Color configuration resolved from CLI flags and environment variables.
#[derive(Debug, Clone, Copy)]
pub(crate) struct ColorConfig {
    enabled: bool,
}

impl ColorConfig {
    /// Create a new ColorConfig from CLI choice.
    ///
    /// Priority: --color flag > NO_COLOR env > terminal detection
    pub fn new(choice: clap::ColorChoice) -> Self {
        let enabled = match choice {
            clap::ColorChoice::Always => true,
            clap::ColorChoice::Never => false,
            clap::ColorChoice::Auto => {
                // Check NO_COLOR environment variable (https://no-color.org/)
                // Standard: "when present and not an empty string"
                if let Ok(val) = std::env::var("NO_COLOR") {
                    if !val.is_empty() {
                        return Self { enabled: false };
                    }
                }

                // Auto: enable color if stdout is a terminal
                std::io::stdout().is_terminal()
            }
        };

        Self { enabled }
    }

    /// Returns whether colors should be used.
    pub fn is_enabled(self) -> bool {
        self.enabled
    }
}

/// Centralized color scheme for consistent CLI output.
///
/// This struct provides semantic color methods for all CLI output.
/// Some methods are currently unused but are kept for:
/// - Future features (new commands, improved output)
/// - API stability (avoiding breaking changes when features are added)
/// - Comprehensive color palette (maintaining consistent theming)
pub(crate) struct ColorScheme;

#[allow(dead_code)]
impl ColorScheme {
    // Operation colors (Creating, Linking, Copying, etc.)
    pub fn operation(text: &str) -> String {
        text.green().to_string()
    }

    pub fn path(text: &str) -> String {
        text.cyan().to_string()
    }

    pub fn skip(text: &str) -> String {
        text.yellow().to_string()
    }

    pub fn remove(text: &str) -> String {
        text.red().to_string()
    }

    // Hook colors
    pub fn hook_type(text: &str) -> String {
        text.magenta().to_string()
    }

    pub fn hook_description(text: &str) -> String {
        text.green().to_string()
    }

    pub fn hook_arrow(text: &str) -> String {
        text.dimmed().to_string()
    }

    pub fn hook_running(text: &str) -> String {
        text.green().to_string()
    }

    // Status colors
    pub fn success_label(text: &str) -> String {
        text.green().to_string()
    }

    pub fn failure_label(text: &str) -> String {
        text.red().to_string()
    }

    pub fn warning(text: &str) -> String {
        text.yellow().bold().to_string()
    }

    pub fn exit_code(text: &str) -> String {
        text.red().to_string()
    }

    // List command colors
    pub fn branch(text: &str) -> String {
        text.magenta().to_string()
    }

    pub fn hash(text: &str) -> String {
        text.yellow().to_string()
    }

    pub fn status(text: &str) -> String {
        text.red().to_string()
    }

    pub fn unpushed(text: &str) -> String {
        text.cyan().to_string()
    }

    pub fn locked(text: &str) -> String {
        text.magenta().to_string()
    }

    // Common
    pub fn dimmed(text: &str) -> String {
        text.dimmed().to_string()
    }
}
