use crossterm::event::KeyEvent;
use ratatui::{
    layout::{Constraint, Direction, Layout},
    style::{Color as RColor, Modifier, Style},
    text::{Line, Span, Text},
    widgets::{Block, Borders, Paragraph, Tabs},
    Frame,
};
use vt100::Parser;

use crate::leader::{KeyAction, LeaderState};
use crate::pty::HarnessConfig;

#[derive(Debug, PartialEq, Clone, Copy, Default)]
pub enum Focus {
    FileTree,
    #[default]
    MainPane,
}

#[derive(Debug, PartialEq, Clone)]
pub enum Tab {
    Harness(String),
    File(String),
}

pub struct SplashApp {
    pub config: HarnessConfig,
    pub leader_state: LeaderState,
    pub focus: Focus,
    pub tabs: Vec<Tab>,
    pub active_tab_index: usize,
    pub raw_output: String,
    pub terminal_size: (u16, u16),
    pub parser: Parser,
}

impl SplashApp {
    pub fn new(config: HarnessConfig) -> Self {
        let initial_tab = Tab::Harness(config.command.clone());
        Self {
            config,
            leader_state: LeaderState::default(),
            focus: Focus::MainPane,
            tabs: vec![initial_tab],
            active_tab_index: 0,
            raw_output: String::new(),
            terminal_size: (78, 22),
            parser: Parser::new(22, 78, 1000),
        }
    }

    pub fn render(&mut self, frame: &mut Frame) {
        let size = frame.size();

        // Top: Tab Bar (1 line), Bottom: Workspace area
        let vertical_chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Length(1), Constraint::Min(0)])
            .split(size);

        let tab_bar_area = vertical_chunks[0];
        let workspace_area = vertical_chunks[1];

        // Render Tab Bar
        let tab_titles: Vec<String> = self
            .tabs
            .iter()
            .enumerate()
            .map(|(i, tab)| match tab {
                Tab::Harness(cmd) => format!(" [{}: {}] ", i + 1, cmd),
                Tab::File(path) => format!(" [{}: {}] ", i + 1, path),
            })
            .collect();

        let tabs_widget = Tabs::new(tab_titles)
            .select(self.active_tab_index)
            .style(Style::default().fg(RColor::DarkGray))
            .highlight_style(Style::default().fg(RColor::Yellow).add_modifier(Modifier::BOLD));

        frame.render_widget(tabs_widget, tab_bar_area);

        // Split workspace: Left = File Tree (~20%), Right = Main Pane (~80%)
        let horizontal_chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(20), Constraint::Percentage(80)])
            .split(workspace_area);

        let file_tree_area = horizontal_chunks[0];
        let main_pane_area = horizontal_chunks[1];

        let tree_border_style = if self.focus == Focus::FileTree {
            Style::default().fg(RColor::Yellow).add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(RColor::DarkGray)
        };

        let main_border_style = if self.focus == Focus::MainPane {
            Style::default().fg(RColor::Yellow).add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(RColor::DarkGray)
        };

        let tree_block = Block::default()
            .title(" File Tree ")
            .borders(Borders::ALL)
            .border_style(tree_border_style);

        let file_tree_paragraph = Paragraph::new("(File tree placeholder)").block(tree_block);
        frame.render_widget(file_tree_paragraph, file_tree_area);

        let leader_active = self.leader_state.is_active();
        let main_title = if leader_active {
            format!(" Main Pane (Harness: {}) [LEADER ACTIVE] ", self.config.command)
        } else {
            format!(" Main Pane (Harness: {}) ", self.config.command)
        };

        let main_block = Block::default()
            .title(main_title)
            .borders(Borders::ALL)
            .border_style(main_border_style);

        let inner_main_area = main_block.inner(main_pane_area);

        self.parser
            .set_size(inner_main_area.height.max(1), inner_main_area.width.max(1));

        let screen = self.parser.screen();
        let text = vt100_screen_to_ratatui_text(screen);
        let main_paragraph = Paragraph::new(text);

        frame.render_widget(main_block, main_pane_area);
        frame.render_widget(main_paragraph, inner_main_area);

        if self.focus == Focus::MainPane && !screen.hide_cursor() {
            let (cursor_row, cursor_col) = screen.cursor_position();
            let target_x = inner_main_area.x + cursor_col;
            let target_y = inner_main_area.y + cursor_row;
            if target_x < inner_main_area.x + inner_main_area.width
                && target_y < inner_main_area.y + inner_main_area.height
            {
                frame.set_cursor(target_x, target_y);
            }
        }
    }

    pub fn handle_key_event(&mut self, key: &KeyEvent) -> KeyAction {
        let action = self.leader_state.handle_key(key);
        match action {
            KeyAction::Quit => KeyAction::Quit,
            KeyAction::FocusFileTree => {
                self.focus = Focus::FileTree;
                KeyAction::None
            }
            KeyAction::FocusMainPane => {
                self.focus = Focus::MainPane;
                KeyAction::None
            }
            KeyAction::SwitchTab(idx) => {
                if idx < self.tabs.len() {
                    self.active_tab_index = idx;
                }
                KeyAction::None
            }
            KeyAction::Forward(bytes) => {
                if self.focus == Focus::MainPane {
                    KeyAction::Forward(bytes)
                } else {
                    KeyAction::None
                }
            }
            KeyAction::None => KeyAction::None,
        }
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
    use crossterm::event::KeyCode;

    #[test]
    fn test_splash_app_initialization_and_mutations() {
        let config = HarnessConfig {
            command: "test".to_string(),
            args: vec![],
        };
        let mut app = SplashApp::new(config);
        assert_eq!(app.terminal_size, (78, 22));
        assert_eq!(app.focus, Focus::MainPane);
        assert_eq!(app.tabs.len(), 1);
        assert_eq!(app.active_tab_index, 0);
        assert!(app.raw_output.is_empty());
        assert!(!app.leader_state.is_active());

        app.push_output_chunk("hello world");
        assert_eq!(app.raw_output, "hello world");

        app.set_size(120, 40);
        assert_eq!(app.terminal_size, (120, 40));
    }

    #[test]
    fn test_focus_switching_and_input_blocking() {
        let config = HarnessConfig {
            command: "bash".to_string(),
            args: vec![],
        };
        let mut app = SplashApp::new(config);

        // Initially MainPane focused
        let key_a = KeyEvent::new(KeyCode::Char('a'), crossterm::event::KeyModifiers::empty());
        assert_eq!(app.handle_key_event(&key_a), KeyAction::Forward(vec![b'a']));

        // Press Ctrl+B Left -> Focus FileTree
        let key_ctrl_b = KeyEvent::new(KeyCode::Char('b'), crossterm::event::KeyModifiers::CONTROL);
        let key_left = KeyEvent::new(KeyCode::Left, crossterm::event::KeyModifiers::empty());
        app.handle_key_event(&key_ctrl_b);
        assert_eq!(app.handle_key_event(&key_left), KeyAction::None);
        assert_eq!(app.focus, Focus::FileTree);

        // When FileTree focused, character inputs are NOT forwarded to PTY
        assert_eq!(app.handle_key_event(&key_a), KeyAction::None);

        // Press Ctrl+B Right -> Focus MainPane
        let key_right = KeyEvent::new(KeyCode::Right, crossterm::event::KeyModifiers::empty());
        app.handle_key_event(&key_ctrl_b);
        assert_eq!(app.handle_key_event(&key_right), KeyAction::None);
        assert_eq!(app.focus, Focus::MainPane);

        // When MainPane focused again, character inputs are forwarded
        assert_eq!(app.handle_key_event(&key_a), KeyAction::Forward(vec![b'a']));
    }

    #[test]
    fn test_tab_switching() {
        let config = HarnessConfig {
            command: "bash".to_string(),
            args: vec![],
        };
        let mut app = SplashApp::new(config);
        app.tabs.push(Tab::File("main.rs".to_string()));

        let key_ctrl_b = KeyEvent::new(KeyCode::Char('b'), crossterm::event::KeyModifiers::CONTROL);
        let key_2 = KeyEvent::new(KeyCode::Char('2'), crossterm::event::KeyModifiers::empty());
        app.handle_key_event(&key_ctrl_b);
        app.handle_key_event(&key_2);

        assert_eq!(app.active_tab_index, 1);
    }
}
