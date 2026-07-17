use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::{
    backend::TestBackend,
    buffer::Buffer,
    Terminal,
};

use crate::app::SplashApp;
use crate::leader::KeyAction;
use crate::pty::HarnessConfig;

pub mod snapshot;
pub use snapshot::*;

pub struct TestHarness {
    pub terminal: Terminal<TestBackend>,
    pub app: SplashApp,
}

impl TestHarness {
    pub fn new(width: u16, height: u16, config: HarnessConfig) -> Self {
        let backend = TestBackend::new(width, height);
        let terminal = Terminal::new(backend).expect("Failed to create TestBackend terminal");
        let mut app = SplashApp::new(config);
        app.set_size(width, height);
        Self { terminal, app }
    }

    pub fn send_key(&mut self, code: KeyCode, modifiers: KeyModifiers) -> KeyAction {
        let key = KeyEvent::new(code, modifiers);
        self.app.handle_key_event(&key)
    }

    pub fn press_char(&mut self, c: char) -> KeyAction {
        self.send_key(KeyCode::Char(c), KeyModifiers::empty())
    }

    pub fn press_ctrl(&mut self, c: char) -> KeyAction {
        self.send_key(KeyCode::Char(c), KeyModifiers::CONTROL)
    }

    pub fn inject_pty_output(&mut self, text: &str) {
        self.app.push_output_chunk(text);
    }

    pub fn resize(&mut self, width: u16, height: u16) {
        self.terminal.backend_mut().resize(width, height);
        self.app.set_size(width, height);
    }

    pub fn render_frame(&mut self) -> &Buffer {
        let app = &mut self.app;
        self.terminal.draw(|f| app.render(f)).unwrap();
        self.terminal.backend().buffer()
    }

    pub fn buffer_snapshot(&mut self) -> String {
        let buffer = self.render_frame();
        format_buffer_grid(buffer)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_harness_creation_and_render() {
        let config = HarnessConfig {
            command: "echo".to_string(),
            args: vec!["test".to_string()],
        };
        let mut harness = TestHarness::new(80, 24, config);
        harness.inject_pty_output("Hello World");
        let buffer = harness.render_frame();
        assert_eq!(buffer.area.width, 80);
        assert_eq!(buffer.area.height, 24);
    }
}
