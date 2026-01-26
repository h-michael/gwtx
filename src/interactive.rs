use crate::error::{Error, Result};
use crate::{config, git};

use crossterm::ExecutableCommand;
use crossterm::event::{self, Event, KeyEvent, KeyEventKind};
use crossterm::terminal::{
    EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode,
};
use ratatui::Terminal;
use ratatui::backend::CrosstermBackend;
use ratatui::style::{Color, Style};

use std::fs::OpenOptions;
use std::io::Write;
use std::time::Duration;

mod add;
mod conflict;
mod path;
mod remove;
mod select;
mod worktree_list;

pub(crate) use add::{AddInteractiveInput, WorktreeSummary, run_add_interactive};
pub(crate) use conflict::{ConflictChoice, prompt_conflict};
pub(crate) use path::run_path_interactive;
pub(crate) use remove::{SafetyWarning, run_remove_confirmation, run_remove_selection};

// Shared step name constants for breadcrumb navigation
const STEP_ACTION: &str = "Choose action";
const STEP_BRANCH: &str = "Select branch";
const STEP_BASE: &str = "Branch from";
const STEP_COMMIT: &str = "Commit hash";
const STEP_BRANCH_NAME: &str = "Branch name";
const STEP_WORKTREE_PATH: &str = "Worktree path";
const STEP_SELECT_WORKTREE: &str = "Select worktrees";
const STEP_CONFIRM: &str = "Confirm";
const STEP_CONFLICT: &str = "Resolve conflict";

// Shared layout constants
const HEADER_HEIGHT: u16 = 2;
const BODY_PADDING: u16 = 1;

/// Renders a breadcrumb line for the header.
///
/// Format: `{command_name}: crumb1 > crumb2 > crumb3`
/// The last crumb is highlighted with accent color.
fn render_breadcrumb_line<'a>(
    command_name: &'a str,
    breadcrumbs: &[&'a str],
    theme: UiTheme,
) -> ratatui::text::Line<'a> {
    use ratatui::text::{Line, Span};

    let mut spans = vec![Span::styled(
        format!("{command_name}: "),
        theme.header_style(),
    )];
    for (i, crumb) in breadcrumbs.iter().enumerate() {
        if i > 0 {
            spans.push(Span::styled(" > ", theme.muted_style()));
        }
        if i == breadcrumbs.len() - 1 {
            spans.push(Span::styled(*crumb, theme.accent_style()));
        } else {
            spans.push(Span::styled(*crumb, theme.header_style()));
        }
    }
    Line::from(spans)
}

fn resolve_ui_theme() -> Result<UiTheme> {
    let repo_root = git::repository_root()?;
    let config = config::load(&repo_root)?.unwrap_or_default();
    Ok(UiTheme::from_colors(&config.ui.colors))
}

/// Reads the next key event from the terminal, filtering out non-key events.
///
/// Returns `Ok(Some(key))` when a key press or repeat event is received,
/// `Ok(None)` on timeout, or `Err` on I/O failure.
///
/// # Filtering behavior
/// - `KeyEventKind::Press` and `KeyEventKind::Repeat`: Returned to caller
/// - `KeyEventKind::Release`: Filtered out (a single key press produces both Press and Release
///   events; processing both would trigger duplicate actions)
/// - Non-key events (mouse, resize, etc.): Filtered out
///
/// # Note
/// This function is not unit-tested because it depends on terminal I/O. The filtering
/// logic is intentionally simple to minimize the risk of bugs.
pub(crate) fn read_key_event(timeout: Duration) -> Result<Option<KeyEvent>> {
    let mut poll_timeout = timeout;
    loop {
        if !event::poll(poll_timeout).map_err(|e| Error::Selector {
            message: format!("Failed to read input: {e}"),
        })? {
            return Ok(None);
        }
        let event = event::read().map_err(|e| Error::Selector {
            message: format!("Failed to read input: {e}"),
        })?;
        if let Event::Key(key) = event {
            if key.kind == KeyEventKind::Release {
                // After filtering an event, poll immediately for the next one without
                // resetting the overall timeout (avoids blocking on filtered events).
                poll_timeout = Duration::from_millis(0);
                continue;
            }
            return Ok(Some(key));
        }
        // Non-key event received; continue polling immediately.
        poll_timeout = Duration::from_millis(0);
    }
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct UiTheme {
    border: Color,
    text: Color,
    accent: Color,
    header: Color,
    footer: Color,
    title: Color,
    label: Color,
    muted: Color,
    disabled: Color,
    search: Color,
    preview: Color,
    selection_bg: Color,
    selection_fg: Color,
    warning: Color,
    error: Color,
}

impl UiTheme {
    pub(crate) fn from_colors(colors: &config::UiColors) -> Self {
        let mut theme = UiTheme::default();
        if let Some(color) = colors.border {
            theme.border = map_ui_color(color);
        }
        if let Some(color) = colors.text {
            theme.text = map_ui_color(color);
        }
        if let Some(color) = colors.accent {
            theme.accent = map_ui_color(color);
        }
        if let Some(color) = colors.header {
            theme.header = map_ui_color(color);
        }
        if let Some(color) = colors.footer {
            theme.footer = map_ui_color(color);
        }
        if let Some(color) = colors.title {
            theme.title = map_ui_color(color);
        }
        if let Some(color) = colors.label {
            theme.label = map_ui_color(color);
        }
        if let Some(color) = colors.muted {
            theme.muted = map_ui_color(color);
        }
        if let Some(color) = colors.disabled {
            theme.disabled = map_ui_color(color);
        }
        if let Some(color) = colors.search {
            theme.search = map_ui_color(color);
        }
        if let Some(color) = colors.preview {
            theme.preview = map_ui_color(color);
        }
        if let Some(color) = colors.selection_bg {
            theme.selection_bg = map_ui_color(color);
        }
        if let Some(color) = colors.selection_fg {
            theme.selection_fg = map_ui_color(color);
        }
        if let Some(color) = colors.warning {
            theme.warning = map_ui_color(color);
        }
        if let Some(color) = colors.error {
            theme.error = map_ui_color(color);
        }
        theme
    }

    fn text_style(self) -> Style {
        Style::default().fg(self.text)
    }

    fn header_style(self) -> Style {
        Style::default().fg(self.header)
    }

    fn footer_style(self) -> Style {
        Style::default().fg(self.footer)
    }

    fn title_style(self) -> Style {
        Style::default().fg(self.title)
    }

    fn label_style(self) -> Style {
        Style::default().fg(self.label)
    }

    fn disabled_style(self) -> Style {
        Style::default().fg(self.disabled)
    }

    fn muted_style(self) -> Style {
        Style::default().fg(self.muted)
    }

    fn accent_style(self) -> Style {
        Style::default().fg(self.accent)
    }

    fn search_style(self) -> Style {
        Style::default().fg(self.search)
    }

    fn preview_style(self) -> Style {
        Style::default().fg(self.preview)
    }

    fn border_style(self) -> Style {
        Style::default().fg(self.border)
    }

    fn selection_style(self) -> Style {
        Style::default().fg(self.selection_fg).bg(self.selection_bg)
    }

    fn warning_style(self) -> Style {
        Style::default().fg(self.warning)
    }

    fn error_style(self) -> Style {
        Style::default().fg(self.error)
    }
}

impl Default for UiTheme {
    fn default() -> Self {
        Self {
            border: Color::DarkGray,
            text: Color::White,
            accent: Color::Cyan,
            header: Color::LightCyan,
            footer: Color::Gray,
            title: Color::LightBlue,
            label: Color::LightMagenta,
            muted: Color::DarkGray,
            disabled: Color::DarkGray,
            search: Color::LightGreen,
            preview: Color::White,
            selection_bg: Color::Blue,
            selection_fg: Color::Black,
            warning: Color::Yellow,
            error: Color::Red,
        }
    }
}

fn map_ui_color(color: config::UiColor) -> Color {
    match color {
        config::UiColor::Rgb(r, g, b) => Color::Rgb(r, g, b),
        config::UiColor::Named(name) => match name {
            config::UiColorName::Default => Color::Reset,
            config::UiColorName::Black => Color::Black,
            config::UiColorName::Red => Color::Red,
            config::UiColorName::Green => Color::Green,
            config::UiColorName::Yellow => Color::Yellow,
            config::UiColorName::Blue => Color::Blue,
            config::UiColorName::Magenta => Color::Magenta,
            config::UiColorName::Cyan => Color::Cyan,
            config::UiColorName::Gray => Color::Gray,
            config::UiColorName::DarkGray => Color::DarkGray,
            config::UiColorName::LightRed => Color::LightRed,
            config::UiColorName::LightGreen => Color::LightGreen,
            config::UiColorName::LightYellow => Color::LightYellow,
            config::UiColorName::LightBlue => Color::LightBlue,
            config::UiColorName::LightMagenta => Color::LightMagenta,
            config::UiColorName::LightCyan => Color::LightCyan,
            config::UiColorName::White => Color::White,
        },
    }
}

fn with_terminal<F, T>(f: F) -> Result<T>
where
    F: FnOnce(&mut Terminal<CrosstermBackend<Box<dyn Write>>>) -> Result<T>,
{
    let mut guard = TerminalGuard::new()?;
    let result = f(guard.terminal_mut());
    let restore_result = guard.restore();
    if let (Err(_), Err(restore_err)) = (&result, &restore_result) {
        // Log restore failure when f() also failed (both errors occurred)
        eprintln!("Warning: Failed to restore terminal: {restore_err}");
    }
    // Propagate f()'s error if it failed, otherwise propagate restore error
    result.and_then(|v| restore_result.map(|()| v))
}

fn setup_terminal() -> Result<Terminal<CrosstermBackend<Box<dyn Write>>>> {
    enable_raw_mode().map_err(|e| Error::Selector {
        message: format!("Failed to enable raw mode: {e}"),
    })?;

    let writer = select_writer().inspect_err(|_| {
        let _ = disable_raw_mode();
    })?;
    let backend = CrosstermBackend::new(writer);
    let mut terminal = Terminal::new(backend).map_err(|e| {
        let _ = disable_raw_mode();
        Error::Selector {
            message: format!("Failed to initialize terminal: {e}"),
        }
    })?;

    if let Err(err) = terminal.backend_mut().execute(EnterAlternateScreen) {
        let _ = disable_raw_mode();
        return Err(Error::Selector {
            message: format!("Failed to enter alternate screen: {err}"),
        });
    }

    if let Err(err) = terminal.clear() {
        let _ = terminal.backend_mut().execute(LeaveAlternateScreen);
        let _ = disable_raw_mode();
        return Err(Error::Selector {
            message: format!("Failed to clear terminal: {err}"),
        });
    }

    Ok(terminal)
}

fn restore_terminal(terminal: &mut Terminal<CrosstermBackend<Box<dyn Write>>>) -> Result<()> {
    terminal
        .backend_mut()
        .execute(LeaveAlternateScreen)
        .map_err(|e| Error::Selector {
            message: format!("Failed to leave alternate screen: {e}"),
        })?;
    disable_raw_mode().map_err(|e| Error::Selector {
        message: format!("Failed to disable raw mode: {e}"),
    })?;
    terminal.show_cursor().map_err(|e| Error::Selector {
        message: format!("Failed to restore cursor: {e}"),
    })?;
    Ok(())
}

struct TerminalGuard {
    terminal: Terminal<CrosstermBackend<Box<dyn Write>>>,
    restored: bool,
}

impl TerminalGuard {
    fn new() -> Result<Self> {
        let terminal = setup_terminal()?;
        Ok(Self {
            terminal,
            restored: false,
        })
    }

    fn terminal_mut(&mut self) -> &mut Terminal<CrosstermBackend<Box<dyn Write>>> {
        &mut self.terminal
    }

    fn restore(&mut self) -> Result<()> {
        if self.restored {
            return Ok(());
        }
        self.restored = true;
        restore_terminal(&mut self.terminal)
    }
}

impl Drop for TerminalGuard {
    fn drop(&mut self) {
        let _ = self.restore();
    }
}

fn select_writer() -> Result<Box<dyn Write>> {
    #[cfg(unix)]
    {
        let file = OpenOptions::new()
            .read(true)
            .write(true)
            .open("/dev/tty")
            .map_err(|e| {
                // Check raw errno for non-interactive conditions:
                // - ENOENT: /dev/tty doesn't exist
                // - ENXIO: No controlling terminal
                // - ENOTTY: Not a terminal device
                match e.raw_os_error() {
                    Some(libc::ENOENT | libc::ENXIO | libc::ENOTTY) => Error::NonInteractive,
                    _ => Error::Internal(format!("Failed to open /dev/tty: {e}")),
                }
            })?;
        Ok(Box::new(file))
    }
    #[cfg(windows)]
    {
        Ok(Box::new(std::io::stdout()))
    }
}

fn truncate_text_for_width(text: String, width: u16) -> String {
    if width == 0 {
        return String::new();
    }
    let max = width as usize;
    let mut out = String::new();
    for (i, line) in text.lines().enumerate() {
        if i > 0 {
            out.push('\n');
        }
        let mut current_width = 0usize;
        for ch in line.chars() {
            // Skip control characters (width returns None for them)
            let Some(char_width) = unicode_width::UnicodeWidthChar::width(ch) else {
                continue;
            };
            if current_width + char_width > max {
                break;
            }
            out.push(ch);
            current_width += char_width;
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{UiColor, UiColorName, UiColors};

    #[test]
    fn test_ui_theme_default() {
        let theme = UiTheme::default();
        assert_eq!(theme.border, Color::DarkGray);
        assert_eq!(theme.text, Color::White);
        assert_eq!(theme.accent, Color::Cyan);
        assert_eq!(theme.header, Color::LightCyan);
        assert_eq!(theme.footer, Color::Gray);
        assert_eq!(theme.title, Color::LightBlue);
        assert_eq!(theme.label, Color::LightMagenta);
        assert_eq!(theme.muted, Color::DarkGray);
        assert_eq!(theme.disabled, Color::DarkGray);
        assert_eq!(theme.search, Color::LightGreen);
        assert_eq!(theme.preview, Color::White);
        assert_eq!(theme.selection_bg, Color::Blue);
        assert_eq!(theme.selection_fg, Color::Black);
        assert_eq!(theme.warning, Color::Yellow);
        assert_eq!(theme.error, Color::Red);
    }

    #[test]
    fn test_ui_theme_from_colors_partial() {
        let colors = UiColors {
            border: Some(UiColor::Named(UiColorName::Red)),
            accent: Some(UiColor::Rgb(255, 128, 0)),
            ..Default::default()
        };
        let theme = UiTheme::from_colors(&colors);

        assert_eq!(theme.border, Color::Red);
        assert_eq!(theme.accent, Color::Rgb(255, 128, 0));
        // Unset colors should use defaults
        assert_eq!(theme.text, Color::White);
        assert_eq!(theme.header, Color::LightCyan);
    }

    #[test]
    fn test_map_ui_color_named() {
        assert_eq!(
            map_ui_color(UiColor::Named(UiColorName::Default)),
            Color::Reset
        );
        assert_eq!(
            map_ui_color(UiColor::Named(UiColorName::Black)),
            Color::Black
        );
        assert_eq!(map_ui_color(UiColor::Named(UiColorName::Red)), Color::Red);
        assert_eq!(
            map_ui_color(UiColor::Named(UiColorName::Green)),
            Color::Green
        );
        assert_eq!(
            map_ui_color(UiColor::Named(UiColorName::Yellow)),
            Color::Yellow
        );
        assert_eq!(map_ui_color(UiColor::Named(UiColorName::Blue)), Color::Blue);
        assert_eq!(
            map_ui_color(UiColor::Named(UiColorName::Magenta)),
            Color::Magenta
        );
        assert_eq!(map_ui_color(UiColor::Named(UiColorName::Cyan)), Color::Cyan);
        assert_eq!(map_ui_color(UiColor::Named(UiColorName::Gray)), Color::Gray);
        assert_eq!(
            map_ui_color(UiColor::Named(UiColorName::DarkGray)),
            Color::DarkGray
        );
        assert_eq!(
            map_ui_color(UiColor::Named(UiColorName::LightRed)),
            Color::LightRed
        );
        assert_eq!(
            map_ui_color(UiColor::Named(UiColorName::LightGreen)),
            Color::LightGreen
        );
        assert_eq!(
            map_ui_color(UiColor::Named(UiColorName::LightYellow)),
            Color::LightYellow
        );
        assert_eq!(
            map_ui_color(UiColor::Named(UiColorName::LightBlue)),
            Color::LightBlue
        );
        assert_eq!(
            map_ui_color(UiColor::Named(UiColorName::LightMagenta)),
            Color::LightMagenta
        );
        assert_eq!(
            map_ui_color(UiColor::Named(UiColorName::LightCyan)),
            Color::LightCyan
        );
        assert_eq!(
            map_ui_color(UiColor::Named(UiColorName::White)),
            Color::White
        );
    }

    #[test]
    fn test_map_ui_color_rgb() {
        assert_eq!(map_ui_color(UiColor::Rgb(255, 0, 0)), Color::Rgb(255, 0, 0));
        assert_eq!(map_ui_color(UiColor::Rgb(0, 255, 0)), Color::Rgb(0, 255, 0));
        assert_eq!(map_ui_color(UiColor::Rgb(0, 0, 255)), Color::Rgb(0, 0, 255));
        assert_eq!(
            map_ui_color(UiColor::Rgb(128, 128, 128)),
            Color::Rgb(128, 128, 128)
        );
    }

    #[test]
    fn test_truncate_text_for_width_basic() {
        assert_eq!(truncate_text_for_width("hello".to_string(), 10), "hello");
        assert_eq!(
            truncate_text_for_width("hello world".to_string(), 5),
            "hello"
        );
        assert_eq!(truncate_text_for_width("hello".to_string(), 3), "hel");
    }

    #[test]
    fn test_truncate_text_for_width_zero() {
        assert_eq!(truncate_text_for_width("hello".to_string(), 0), "");
    }

    #[test]
    fn test_truncate_text_for_width_multiline() {
        let text = "line one\nline two\nline three".to_string();
        let result = truncate_text_for_width(text, 4);
        assert_eq!(result, "line\nline\nline");
    }

    #[test]
    fn test_truncate_text_for_width_unicode() {
        // CJK characters have display width of 2 each
        // width=6 can fit 3 CJK characters (3 * 2 = 6)
        assert_eq!(
            truncate_text_for_width("日本語テスト".to_string(), 6),
            "日本語"
        );
        // width=3 can only fit 1 CJK character (1 * 2 = 2, next would be 4 > 3)
        assert_eq!(truncate_text_for_width("日本語テスト".to_string(), 3), "日");
        // Greek letters have display width of 1 each (like ASCII)
        assert_eq!(truncate_text_for_width("αβγδε".to_string(), 3), "αβγ");
    }

    #[test]
    fn test_truncate_text_for_width_control_chars() {
        // Control characters (ESC, NUL, etc.) are stripped
        // Note: Only the control char itself is removed, not the entire ANSI sequence
        assert_eq!(truncate_text_for_width("a\x1bb".to_string(), 2), "ab");
        assert_eq!(truncate_text_for_width("a\x00b\x01c".to_string(), 3), "abc");
        // Bell and backspace stripped
        assert_eq!(truncate_text_for_width("a\x07b\x08c".to_string(), 3), "abc");
    }

    #[test]
    fn test_ui_theme_styles() {
        let theme = UiTheme::default();

        // Verify style methods return expected styles
        assert_eq!(theme.text_style().fg, Some(Color::White));
        assert_eq!(theme.header_style().fg, Some(Color::LightCyan));
        assert_eq!(theme.footer_style().fg, Some(Color::Gray));
        assert_eq!(theme.title_style().fg, Some(Color::LightBlue));
        assert_eq!(theme.label_style().fg, Some(Color::LightMagenta));
        assert_eq!(theme.disabled_style().fg, Some(Color::DarkGray));
        assert_eq!(theme.search_style().fg, Some(Color::LightGreen));
        assert_eq!(theme.preview_style().fg, Some(Color::White));
        assert_eq!(theme.border_style().fg, Some(Color::DarkGray));
        assert_eq!(theme.warning_style().fg, Some(Color::Yellow));
        assert_eq!(theme.error_style().fg, Some(Color::Red));

        // Selection style has both fg and bg
        let selection = theme.selection_style();
        assert_eq!(selection.fg, Some(Color::Black));
        assert_eq!(selection.bg, Some(Color::Blue));
    }
}
