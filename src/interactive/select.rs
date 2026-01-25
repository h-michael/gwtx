use crate::error::{Error, Result};

use crossterm::event::{KeyCode, KeyModifiers};
use ratatui::layout::{Constraint, Direction, Layout};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, List, ListItem, Padding, Paragraph, Wrap};

use std::time::Duration;

use super::{UiTheme, read_key_event, truncate_text_for_width, with_terminal};

pub(crate) fn select_from_list(
    title: &str,
    message: Option<&str>,
    items: &[String],
    theme: UiTheme,
) -> Result<String> {
    if items.is_empty() {
        return Err(Error::Selector {
            message: "No items to select".to_string(),
        });
    }

    with_terminal(|terminal| run_simple_select(terminal, title, message, items, theme))
}

pub(crate) fn confirm(
    command_name: &str,
    breadcrumbs: &[&str],
    message: &str,
    details: &[String],
    theme: UiTheme,
) -> Result<bool> {
    with_terminal(|terminal| {
        run_confirm(terminal, command_name, breadcrumbs, message, details, theme)
    })
}

struct SimpleSelectState {
    cursor: usize,
}

impl SimpleSelectState {
    fn new() -> Self {
        Self { cursor: 0 }
    }

    fn move_up(&mut self) {
        if self.cursor > 0 {
            self.cursor -= 1;
        }
    }

    fn move_down(&mut self, len: usize) {
        if len == 0 {
            return;
        }
        if self.cursor + 1 < len {
            self.cursor += 1;
        }
    }
}

fn run_simple_select(
    terminal: &mut ratatui::Terminal<ratatui::backend::CrosstermBackend<Box<dyn std::io::Write>>>,
    title: &str,
    message: Option<&str>,
    items: &[String],
    theme: UiTheme,
) -> Result<String> {
    let mut state = SimpleSelectState::new();

    loop {
        terminal
            .draw(|frame| draw_simple_select(frame, &state, title, message, items, theme))
            .map_err(|e| Error::Selector {
                message: format!("Failed to draw UI: {e}"),
            })?;

        if let Some(key) = read_key_event(Duration::from_millis(200))? {
            if key.code == KeyCode::Char('c') && key.modifiers.contains(KeyModifiers::CONTROL) {
                return Err(Error::Aborted);
            }
            match key.code {
                KeyCode::Esc => return Err(Error::Aborted),
                KeyCode::Enter => {
                    return items.get(state.cursor).cloned().ok_or(Error::Aborted);
                }
                KeyCode::Up => state.move_up(),
                KeyCode::Down => state.move_down(items.len()),
                KeyCode::Char(c) if key.modifiers.contains(KeyModifiers::CONTROL) => match c {
                    'p' | 'k' => state.move_up(),
                    'n' | 'j' => state.move_down(items.len()),
                    _ => {}
                },
                _ => {}
            }
        }
    }
}

fn draw_simple_select(
    frame: &mut ratatui::Frame<'_>,
    state: &SimpleSelectState,
    title: &str,
    message: Option<&str>,
    items: &[String],
    theme: UiTheme,
) {
    let size = frame.area();
    let header_height = 2;
    let body_padding = 1;
    let body_height = size.height.saturating_sub(header_height + body_padding);

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(header_height),
            Constraint::Length(body_padding),
            Constraint::Length(body_height),
        ])
        .split(size);

    let key_hints = "[Enter] select  [Esc] cancel  [↑/↓] move";
    let title_line = Line::from(Span::styled(title, theme.header_style()));
    let key_hints_line = Line::from(Span::styled(key_hints, theme.footer_style()));
    let header = Paragraph::new(vec![title_line, key_hints_line]);
    frame.render_widget(header, chunks[0]);

    let (message_area, list_area) = if message.is_some() {
        let split = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Length(1), Constraint::Min(1)])
            .split(chunks[2]);
        (Some(split[0]), split[1])
    } else {
        (None, chunks[2])
    };

    if let (Some(message), Some(area)) = (message, message_area) {
        let message = truncate_text_for_width(message.to_string(), area.width);
        let message = Paragraph::new(message).style(theme.label_style());
        frame.render_widget(message, area);
    }

    let items = items
        .iter()
        .enumerate()
        .map(|(idx, item)| {
            let mut list_item = ListItem::new(truncate_text_for_width(
                item.clone(),
                list_area.width.saturating_sub(2),
            ));
            if idx == state.cursor {
                list_item = list_item.style(theme.selection_style());
            } else {
                list_item = list_item.style(theme.text_style());
            }
            list_item
        })
        .collect::<Vec<_>>();

    let list = List::new(items)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(theme.border_style())
                .padding(Padding::new(1, 1, 0, 0))
                .title(Span::styled("Options", theme.title_style())),
        )
        .style(theme.text_style());
    frame.render_widget(list, list_area);
}

fn run_confirm(
    terminal: &mut ratatui::Terminal<ratatui::backend::CrosstermBackend<Box<dyn std::io::Write>>>,
    command_name: &str,
    breadcrumbs: &[&str],
    message: &str,
    details: &[String],
    theme: UiTheme,
) -> Result<bool> {
    loop {
        terminal
            .draw(|frame| {
                draw_confirm_dialog(frame, command_name, breadcrumbs, message, details, theme)
            })
            .map_err(|e| Error::Selector {
                message: format!("Failed to draw UI: {e}"),
            })?;

        if let Some(key) = read_key_event(Duration::from_millis(200))? {
            if key.code == KeyCode::Char('c') && key.modifiers.contains(KeyModifiers::CONTROL) {
                return Err(Error::Aborted);
            }
            match key.code {
                KeyCode::Esc => return Ok(false),
                KeyCode::Enter => return Ok(true),
                KeyCode::Char('y') | KeyCode::Char('Y') => return Ok(true),
                KeyCode::Char('n') | KeyCode::Char('N') => return Ok(false),
                _ => {}
            }
        }
    }
}

fn draw_confirm_dialog(
    frame: &mut ratatui::Frame<'_>,
    command_name: &str,
    breadcrumbs: &[&str],
    message: &str,
    details: &[String],
    theme: UiTheme,
) {
    let size = frame.area();
    let header_height = 2;
    let body_padding = 1;
    let body_height = size.height.saturating_sub(header_height + body_padding);

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(header_height),
            Constraint::Length(body_padding),
            Constraint::Length(body_height),
        ])
        .split(size);

    let key_hints = "[Enter] yes  [N] no  [Esc] cancel";

    let mut title_spans = vec![Span::styled(
        format!("{command_name}: "),
        theme.header_style(),
    )];
    for (i, crumb) in breadcrumbs.iter().enumerate() {
        if i > 0 {
            title_spans.push(Span::styled(" > ", theme.muted_style()));
        }
        if i == breadcrumbs.len() - 1 {
            title_spans.push(Span::styled(*crumb, theme.accent_style()));
        } else {
            title_spans.push(Span::styled(*crumb, theme.header_style()));
        }
    }
    let title_line = Line::from(title_spans);
    let key_hints_line = Line::from(Span::styled(key_hints, theme.footer_style()));
    let header = Paragraph::new(vec![title_line, key_hints_line]);
    frame.render_widget(header, chunks[0]);

    let box_title = breadcrumbs.last().copied().unwrap_or("Confirm");
    let mut lines = Vec::new();
    lines.push(Line::from(Span::styled(
        message.to_string(),
        theme.label_style(),
    )));
    if !details.is_empty() {
        lines.push(Line::from(Span::raw("")));
        lines.extend(
            details
                .iter()
                .map(|line| Line::from(Span::styled(line.clone(), theme.text_style()))),
        );
    }

    let body = Paragraph::new(lines)
        .style(theme.text_style())
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(theme.border_style())
                .padding(Padding::new(1, 1, 0, 0))
                .title(Span::styled(box_title, theme.title_style())),
        )
        .wrap(Wrap { trim: true });
    frame.render_widget(body, chunks[2]);
}

#[cfg(test)]
mod tests {
    use super::*;
    use ratatui::Terminal;
    use ratatui::backend::TestBackend;

    #[test]
    fn test_simple_select_state_new() {
        let state = SimpleSelectState::new();
        assert_eq!(state.cursor, 0);
    }

    #[test]
    fn test_simple_select_state_move_up() {
        let mut state = SimpleSelectState::new();
        state.cursor = 2;

        state.move_up();
        assert_eq!(state.cursor, 1);

        state.move_up();
        assert_eq!(state.cursor, 0);

        // Should not go below 0
        state.move_up();
        assert_eq!(state.cursor, 0);
    }

    #[test]
    fn test_simple_select_state_move_down() {
        let mut state = SimpleSelectState::new();
        let len = 3;

        state.move_down(len);
        assert_eq!(state.cursor, 1);

        state.move_down(len);
        assert_eq!(state.cursor, 2);

        // Should not exceed len - 1
        state.move_down(len);
        assert_eq!(state.cursor, 2);
    }

    #[test]
    fn test_simple_select_state_move_down_empty() {
        let mut state = SimpleSelectState::new();

        // Should not panic or change cursor when len is 0
        state.move_down(0);
        assert_eq!(state.cursor, 0);
    }

    #[test]
    fn test_draw_simple_select_renders() {
        let backend = TestBackend::new(80, 24);
        let mut terminal = Terminal::new(backend).unwrap();
        let state = SimpleSelectState::new();
        let items = vec!["Option 1".to_string(), "Option 2".to_string()];
        let theme = UiTheme::default();

        terminal
            .draw(|frame| {
                draw_simple_select(frame, &state, "Test Title", None, &items, theme);
            })
            .unwrap();

        let buffer = terminal.backend().buffer();
        // Verify title is rendered (header starts at row 0 after removing top padding)
        let title_str: String = (0..10)
            .map(|x| {
                buffer
                    .cell((x, 0))
                    .unwrap()
                    .symbol()
                    .chars()
                    .next()
                    .unwrap_or(' ')
            })
            .collect();
        assert!(title_str.contains("Test"));
    }

    #[test]
    fn test_draw_simple_select_with_message() {
        let backend = TestBackend::new(80, 24);
        let mut terminal = Terminal::new(backend).unwrap();
        let state = SimpleSelectState::new();
        let items = vec!["Item A".to_string(), "Item B".to_string()];
        let theme = UiTheme::default();

        terminal
            .draw(|frame| {
                draw_simple_select(
                    frame,
                    &state,
                    "Title",
                    Some("Select an option"),
                    &items,
                    theme,
                );
            })
            .unwrap();

        // Just verify it renders without panic
        assert!(terminal.backend().buffer().area.width > 0);
    }

    #[test]
    fn test_draw_confirm_dialog_renders() {
        let backend = TestBackend::new(80, 24);
        let mut terminal = Terminal::new(backend).unwrap();
        let theme = UiTheme::default();
        let details = vec!["Detail 1".to_string(), "Detail 2".to_string()];

        terminal
            .draw(|frame| {
                draw_confirm_dialog(
                    frame,
                    "test",
                    &["Confirm"],
                    "Are you sure?",
                    &details,
                    theme,
                );
            })
            .unwrap();

        let buffer = terminal.backend().buffer();
        // Verify title is rendered (header starts at row 0 after removing top padding)
        let title_str: String = (0..15)
            .map(|x| {
                buffer
                    .cell((x, 0))
                    .unwrap()
                    .symbol()
                    .chars()
                    .next()
                    .unwrap_or(' ')
            })
            .collect();
        assert!(title_str.contains("Confirm"));
    }

    #[test]
    fn test_draw_confirm_dialog_empty_details() {
        let backend = TestBackend::new(80, 24);
        let mut terminal = Terminal::new(backend).unwrap();
        let theme = UiTheme::default();

        terminal
            .draw(|frame| {
                draw_confirm_dialog(frame, "test", &["Confirm"], "Message", &[], theme);
            })
            .unwrap();

        // Just verify it renders without panic
        assert!(terminal.backend().buffer().area.width > 0);
    }
}
