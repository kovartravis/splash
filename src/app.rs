use crossterm::event::KeyEvent;
use ratatui::{
    style::{Color as RColor, Modifier, Style},
    text::{Line, Span, Text},
    widgets::{Block, Borders, Paragraph},
    Frame,
};
use vt100::Parser;

use crate::leader::{KeyAction, LeaderState};
use crate::pty::HarnessConfig;

pub struct SplashApp {
    pub config: HarnessConfig,
    pub leader_state: LeaderState,
    pub raw_output: String,
    pub terminal_size: (u16, u16),
    pub parser: Parser,
}

impl SplashApp {
    pub fn new(config: HarnessConfig) -> Self {
        Self {
            config,
            leader_state: LeaderState::default(),
            raw_output: String::new(),
            terminal_size: (78, 22),
            parser: Parser::new(22, 78, 1000),
        }
    }

    pub fn render(&mut self, frame: &mut Frame) {
        let rect = frame.size();
        let leader_active = self.leader_state.is_active();
        let cmd_title = format!(
            " Harness: {} (Leader: Ctrl+B | Exit: Ctrl+B q) ",
            self.config.command
        );

        let title = if leader_active {
            format!("{} [LEADER ACTIVE]", cmd_title)
        } else {
            cmd_title
        };

        let block = Block::default().title(title).borders(Borders::ALL);
        let inner_area = block.inner(rect);

        self.parser
            .set_size(inner_area.height.max(1), inner_area.width.max(1));

        let screen = self.parser.screen();
        let text = vt100_screen_to_ratatui_text(screen);
        let paragraph = Paragraph::new(text);

        frame.render_widget(block, rect);
        frame.render_widget(paragraph, inner_area);

        // Position cursor if screen cursor is not hidden
        if !screen.hide_cursor() {
            let (cursor_row, cursor_col) = screen.cursor_position();
            let target_x = inner_area.x + cursor_col;
            let target_y = inner_area.y + cursor_row;
            if target_x < inner_area.x + inner_area.width && target_y < inner_area.y + inner_area.height {
                frame.set_cursor(target_x, target_y);
            }
        }
    }

    pub fn handle_key_event(&mut self, key: &KeyEvent) -> KeyAction {
        self.leader_state.handle_key(key)
    }

    pub fn push_output_chunk(&mut self, text: &str) {
        self.raw_output.push_str(text);
        self.parser.process(text.as_bytes());
    }

    pub fn set_size(&mut self, width: u16, height: u16) {
        self.terminal_size = (width, height);
        self.parser.set_size(height.max(1), width.max(1));
    }
}

pub fn vt100_color_to_ratatui(color: vt100::Color) -> RColor {
    match color {
        vt100::Color::Default => RColor::Reset,
        vt100::Color::Idx(idx) => RColor::Indexed(idx),
        vt100::Color::Rgb(r, g, b) => RColor::Rgb(r, g, b),
    }
}

pub fn vt100_screen_to_ratatui_text(screen: &vt100::Screen) -> Text<'static> {
    let (rows, cols) = screen.size();
    let mut lines = Vec::with_capacity(rows as usize);

    for row in 0..rows {
        let mut spans = Vec::new();
        let mut current_str = String::new();
        let mut current_style = Style::default();

        for col in 0..cols {
            if let Some(cell) = screen.cell(row, col) {
                if cell.is_wide_continuation() {
                    continue;
                }
                let fg = vt100_color_to_ratatui(cell.fgcolor());
                let bg = vt100_color_to_ratatui(cell.bgcolor());
                let mut modifier = Modifier::empty();
                if cell.bold() {
                    modifier |= Modifier::BOLD;
                }
                if cell.italic() {
                    modifier |= Modifier::ITALIC;
                }
                if cell.underline() {
                    modifier |= Modifier::UNDERLINED;
                }

                let style = Style::default().fg(fg).bg(bg).add_modifier(modifier);
                let symbol = cell.contents();

                if style == current_style {
                    current_str.push_str(&symbol);
                } else {
                    if !current_str.is_empty() {
                        spans.push(Span::styled(current_str.clone(), current_style));
                        current_str.clear();
                    }
                    current_style = style;
                    current_str.push_str(&symbol);
                }
            } else {
                current_str.push(' ');
            }
        }
        if !current_str.is_empty() {
            spans.push(Span::styled(current_str, current_style));
        }
        lines.push(Line::from(spans));
    }

    Text::from(lines)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_splash_app_initialization_and_mutations() {
        let config = HarnessConfig {
            command: "test".to_string(),
            args: vec![],
        };
        let mut app = SplashApp::new(config);
        assert_eq!(app.terminal_size, (78, 22));
        assert!(app.raw_output.is_empty());
        assert!(!app.leader_state.is_active());

        app.push_output_chunk("hello world");
        assert_eq!(app.raw_output, "hello world");

        app.set_size(120, 40);
        assert_eq!(app.terminal_size, (120, 40));
    }

    #[test]
    fn test_vt100_parser_handles_carriage_returns_and_ansi_escape() {
        let config = HarnessConfig {
            command: "agy".to_string(),
            args: vec![],
        };
        let mut app = SplashApp::new(config);

        // Push text with carriage return (updating same line)
        app.push_output_chunk("Loading 0%\rLoading 50%\rLoading 100%\nDone!");

        let contents = app.parser.screen().contents();
        assert!(contents.contains("Loading 100%"));
        assert!(contents.contains("Done!"));
        // Confirm intermediate "Loading 0%" was overwritten in 2D buffer
        assert!(!contents.contains("Loading 0%"));
    }

    #[test]
    fn test_vt100_screen_to_ratatui_text_colors() {
        let mut parser = Parser::new(2, 20, 0);
        parser.process("\x1b[31mRed\x1b[0m \x1b[1;32mGreen\x1b[0m".as_bytes());
        let text = vt100_screen_to_ratatui_text(parser.screen());
        assert_eq!(text.lines.len(), 2);
    }
}
