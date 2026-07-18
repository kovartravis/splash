use crossterm::event::KeyEvent;
use ratatui::{
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
        let contents = screen.contents();

        let paragraph = Paragraph::new(contents);

        frame.render_widget(block, rect);
        frame.render_widget(paragraph, inner_area);
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
}
