use crossterm::event::KeyEvent;
use ratatui::{
    widgets::{Block, Borders, Paragraph},
    Frame,
};

use crate::leader::{KeyAction, LeaderState};
use crate::pty::HarnessConfig;

#[derive(Debug, Clone)]
pub struct SplashApp {
    pub config: HarnessConfig,
    pub leader_state: LeaderState,
    pub raw_output: String,
    pub terminal_size: (u16, u16),
}

impl SplashApp {
    pub fn new(config: HarnessConfig) -> Self {
        Self {
            config,
            leader_state: LeaderState::default(),
            raw_output: String::new(),
            terminal_size: (80, 24),
        }
    }

    pub fn render(&mut self, frame: &mut Frame) {
        let lines: Vec<&str> = self.raw_output.lines().collect();
        let display_text = if lines.len() > 100 {
            lines[lines.len() - 100..].join("\n")
        } else {
            self.raw_output.clone()
        };

        let leader_active = self.leader_state.is_active();
        let cmd_title = format!(" Harness: {} (Leader: Ctrl+B | Exit: Ctrl+B q) ", self.config.command);

        let title = if leader_active {
            format!("{} [LEADER ACTIVE]", cmd_title)
        } else {
            cmd_title
        };

        let rect = frame.size();
        let block = Block::default().title(title).borders(Borders::ALL);
        let paragraph = Paragraph::new(display_text).block(block);
        frame.render_widget(paragraph, rect);
    }

    pub fn handle_key_event(&mut self, key: &KeyEvent) -> KeyAction {
        self.leader_state.handle_key(key)
    }

    pub fn push_output_chunk(&mut self, text: &str) {
        self.raw_output.push_str(text);
    }

    pub fn set_size(&mut self, width: u16, height: u16) {
        self.terminal_size = (width, height);
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
        assert_eq!(app.terminal_size, (80, 24));
        assert!(app.raw_output.is_empty());
        assert!(!app.leader_state.is_active());

        app.push_output_chunk("hello world");
        assert_eq!(app.raw_output, "hello world");

        app.set_size(120, 40);
        assert_eq!(app.terminal_size, (120, 40));
    }
}
