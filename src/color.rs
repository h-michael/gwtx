use std::io::IsTerminal;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Mutex, OnceLock};

use owo_colors::{AnsiColors, DynColors, Style};

use crate::config;

static COLOR_ENABLED: AtomicBool = AtomicBool::new(true);

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
                if let Ok(val) = std::env::var("NO_COLOR")
                    && !val.is_empty()
                {
                    return Self::with_enabled(false);
                }

                // Auto: enable color if stdout is a terminal
                std::io::stdout().is_terminal()
            }
        };

        Self::with_enabled(enabled)
    }

    fn with_enabled(enabled: bool) -> Self {
        COLOR_ENABLED.store(enabled, Ordering::Relaxed);
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

impl ColorScheme {
    // Operation colors (Creating, Linking, Copying, etc.)
    pub fn operation(text: &str) -> String {
        apply_style(theme().operation, text)
    }

    pub fn path(text: &str) -> String {
        apply_style(theme().path, text)
    }

    pub fn skip(text: &str) -> String {
        apply_style(theme().skip, text)
    }

    // Hook colors
    pub fn hook_type(text: &str) -> String {
        apply_style(theme().hook_type, text)
    }

    pub fn hook_description(text: &str) -> String {
        apply_style(theme().hook_description, text)
    }

    pub fn hook_arrow(text: &str) -> String {
        apply_style(theme().hook_arrow, text)
    }

    pub fn hook_running(text: &str) -> String {
        apply_style(theme().hook_running, text)
    }

    // Status colors
    pub fn success_label(text: &str) -> String {
        apply_style(theme().success_label, text)
    }

    pub fn failure_label(text: &str) -> String {
        apply_style(theme().failure_label, text)
    }

    pub fn error(text: &str) -> String {
        apply_style(theme().error, text)
    }

    pub fn warning(text: &str) -> String {
        apply_style(theme().warning, text)
    }

    pub fn exit_code(text: &str) -> String {
        apply_style(theme().exit_code, text)
    }

    // List command colors
    pub fn header(text: &str) -> String {
        apply_style(theme().header, text)
    }

    pub fn branch(text: &str) -> String {
        apply_style(theme().branch, text)
    }

    pub fn hash(text: &str) -> String {
        apply_style(theme().hash, text)
    }

    // Diff colors
    pub fn diff_removed(text: &str) -> String {
        apply_style(theme().diff_removed, text)
    }

    pub fn diff_added(text: &str) -> String {
        apply_style(theme().diff_added, text)
    }

    // Common
    pub fn dimmed(text: &str) -> String {
        apply_style(theme().dimmed, text)
    }
}

#[derive(Clone, Copy)]
struct CliTheme {
    operation: Style,
    path: Style,
    skip: Style,
    hook_type: Style,
    hook_description: Style,
    hook_arrow: Style,
    hook_running: Style,
    success_label: Style,
    failure_label: Style,
    error: Style,
    warning: Style,
    exit_code: Style,
    header: Style,
    branch: Style,
    hash: Style,
    diff_removed: Style,
    diff_added: Style,
    dimmed: Style,
}

impl Default for CliTheme {
    fn default() -> Self {
        Self {
            operation: Style::new().color(DynColors::Ansi(AnsiColors::Green)),
            path: Style::new().color(DynColors::Ansi(AnsiColors::Cyan)),
            skip: Style::new().color(DynColors::Ansi(AnsiColors::Yellow)),
            hook_type: Style::new().color(DynColors::Ansi(AnsiColors::Magenta)),
            hook_description: Style::new().color(DynColors::Ansi(AnsiColors::Green)),
            hook_arrow: Style::new().dimmed(),
            hook_running: Style::new().color(DynColors::Ansi(AnsiColors::Green)),
            success_label: Style::new().color(DynColors::Ansi(AnsiColors::Green)),
            failure_label: Style::new().color(DynColors::Ansi(AnsiColors::Red)),
            error: Style::new().color(DynColors::Ansi(AnsiColors::Red)),
            warning: Style::new()
                .color(DynColors::Ansi(AnsiColors::Yellow))
                .bold(),
            exit_code: Style::new().color(DynColors::Ansi(AnsiColors::Red)),
            header: Style::new()
                .color(DynColors::Ansi(AnsiColors::White))
                .bold(),
            branch: Style::new().color(DynColors::Ansi(AnsiColors::Magenta)),
            hash: Style::new().color(DynColors::Ansi(AnsiColors::Yellow)),
            diff_removed: Style::new().color(DynColors::Ansi(AnsiColors::Red)),
            diff_added: Style::new().color(DynColors::Ansi(AnsiColors::Green)),
            dimmed: Style::new().dimmed(),
        }
    }
}

impl CliTheme {
    fn from_colors(colors: &config::UiColors) -> Self {
        let mut theme = CliTheme::default();
        if let Some(color) = colors.accent {
            let style = Style::new().color(to_dyn_color(color));
            theme.operation = style;
            theme.path = style;
            theme.hook_running = style;
            theme.success_label = style;
            theme.diff_added = style;
        }
        if let Some(color) = colors.warning {
            theme.skip = Style::new().color(to_dyn_color(color));
            theme.warning = Style::new().color(to_dyn_color(color)).bold();
            theme.hash = Style::new().color(to_dyn_color(color));
        }
        if let Some(color) = colors.error {
            theme.failure_label = Style::new().color(to_dyn_color(color));
            theme.error = Style::new().color(to_dyn_color(color));
            theme.exit_code = Style::new().color(to_dyn_color(color));
            theme.diff_removed = Style::new().color(to_dyn_color(color));
        }
        if let Some(color) = colors.title {
            theme.hook_type = Style::new().color(to_dyn_color(color));
        }
        if let Some(color) = colors.label {
            theme.branch = Style::new().color(to_dyn_color(color));
        }
        if let Some(color) = colors.text {
            theme.hook_description = Style::new().color(to_dyn_color(color));
        }
        if let Some(color) = colors.header {
            theme.header = Style::new().color(to_dyn_color(color)).bold();
        }
        if let Some(color) = colors.muted {
            theme.hook_arrow = Style::new().color(to_dyn_color(color)).dimmed();
            theme.dimmed = Style::new().color(to_dyn_color(color)).dimmed();
        }
        theme
    }
}

static CLI_THEME: OnceLock<Mutex<CliTheme>> = OnceLock::new();

pub(crate) fn set_cli_theme(colors: &config::UiColors) {
    let lock = CLI_THEME.get_or_init(|| Mutex::new(CliTheme::default()));
    if let Ok(mut theme) = lock.lock() {
        *theme = CliTheme::from_colors(colors);
    }
}

fn theme() -> CliTheme {
    let lock = CLI_THEME.get_or_init(|| Mutex::new(CliTheme::default()));
    match lock.lock() {
        Ok(theme) => *theme,
        Err(_) => CliTheme::default(),
    }
}

fn apply_style(style: Style, text: &str) -> String {
    if COLOR_ENABLED.load(Ordering::Relaxed) {
        style.style(text).to_string()
    } else {
        text.to_string()
    }
}

fn to_dyn_color(color: config::UiColor) -> DynColors {
    match color {
        config::UiColor::Rgb(r, g, b) => DynColors::Rgb(r, g, b),
        config::UiColor::Named(name) => {
            let ansi = match name {
                config::UiColorName::Default => AnsiColors::Default,
                config::UiColorName::Black => AnsiColors::Black,
                config::UiColorName::Red => AnsiColors::Red,
                config::UiColorName::Green => AnsiColors::Green,
                config::UiColorName::Yellow => AnsiColors::Yellow,
                config::UiColorName::Blue => AnsiColors::Blue,
                config::UiColorName::Magenta => AnsiColors::Magenta,
                config::UiColorName::Cyan => AnsiColors::Cyan,
                config::UiColorName::Gray => AnsiColors::BrightBlack,
                config::UiColorName::DarkGray => AnsiColors::Black,
                config::UiColorName::LightRed => AnsiColors::BrightRed,
                config::UiColorName::LightGreen => AnsiColors::BrightGreen,
                config::UiColorName::LightYellow => AnsiColors::BrightYellow,
                config::UiColorName::LightBlue => AnsiColors::BrightBlue,
                config::UiColorName::LightMagenta => AnsiColors::BrightMagenta,
                config::UiColorName::LightCyan => AnsiColors::BrightCyan,
                config::UiColorName::White => AnsiColors::White,
            };
            DynColors::Ansi(ansi)
        }
    }
}
