use std::io;
use std::path::PathBuf;

use crossterm::event::KeyEvent;
use ratatui::{
    layout::{Constraint, Direction, Layout},
    style::{Color as RColor, Modifier, Style},
    text::{Line, Span, Text},
    widgets::{Block, Borders, Paragraph, Tabs, Wrap},
    Frame,
};
use vt100::Parser;

use crate::leader::{KeyAction, LeaderState};
use crate::pty::HarnessConfig;
use crate::tree::FileTree;

#[derive(Debug, PartialEq, Clone, Copy, Default)]
pub enum Focus {
    FileTree,
    #[default]
    MainPane,
}

#[derive(Debug, PartialEq, Clone)]
pub struct FileTab {
    pub path: PathBuf,
    pub content: String,
    pub scroll_offset: u16,
}

impl FileTab {
    pub fn open(path: impl Into<PathBuf>) -> io::Result<Self> {
        let mut tab = Self {
            path: path.into(),
            content: String::new(),
            scroll_offset: 0,
        };
        tab.reload()?;
        Ok(tab)
    }

    pub fn reload(&mut self) -> io::Result<()> {
        let bytes = std::fs::read(&self.path)?;
        self.content = String::from_utf8_lossy(&bytes).into_owned();
        self.clamp_scroll();
        Ok(())
    }

    pub fn max_scroll_offset(&self) -> u16 {
        let line_count = self.content.lines().count();
        if line_count == 0 {
            0
        } else {
            (line_count - 1) as u16
        }
    }

    pub fn clamp_scroll(&mut self) {
        let max = self.max_scroll_offset();
        if self.scroll_offset > max {
            self.scroll_offset = max;
        }
    }
}

use std::sync::{Arc, Mutex};
use crate::pty::PtyHarness;

#[derive(Clone)]
pub struct HarnessTab {
    pub command: String,
    pub args: Vec<String>,
    pub pty: Option<Arc<Mutex<PtyHarness>>>,
    pub parser: Arc<Mutex<Parser>>,
    pub last_size: Option<(u16, u16)>,
}

impl std::fmt::Debug for HarnessTab {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("HarnessTab")
            .field("command", &self.command)
            .field("args", &self.args)
            .field("has_pty", &self.pty.is_some())
            .finish()
    }
}

impl PartialEq for HarnessTab {
    fn eq(&self, other: &Self) -> bool {
        self.command == other.command && self.args == other.args
    }
}

impl HarnessTab {
    pub fn new(command: impl Into<String>) -> Self {
        Self::with_args(command, vec![])
    }

    pub fn with_args(command: impl Into<String>, args: Vec<String>) -> Self {
        Self {
            command: command.into(),
            args,
            pty: None,
            parser: Arc::new(Mutex::new(Parser::new(22, 78, 1000))),
            last_size: None,
        }
    }

    pub fn with_pty(command: impl Into<String>, pty: PtyHarness, rows: u16, cols: u16) -> Self {
        Self {
            command: command.into(),
            args: vec![],
            pty: Some(Arc::new(Mutex::new(pty))),
            parser: Arc::new(Mutex::new(Parser::new(rows.max(1), cols.max(1), 1000))),
            last_size: Some((rows, cols)),
        }
    }

    pub fn spawn_pty(&mut self, rows: u16, cols: u16) {
        let config = HarnessConfig {
            command: self.command.clone(),
            args: self.args.clone(),
        };
        if let Ok(pty) = PtyHarness::spawn(&config, rows, cols) {
            self.pty = Some(Arc::new(Mutex::new(pty)));
        }
    }

    pub fn tick(&self) {
        if let Some(ref pty) = self.pty {
            if let Ok(guard) = pty.lock() {
                while let Ok(chunk) = guard.output_rx.try_recv() {
                    if let Ok(mut parser) = self.parser.lock() {
                        parser.process(chunk.as_bytes());
                    }
                }
            }
        }
    }

    pub fn write(&self, bytes: &[u8]) {
        if let Some(ref pty) = self.pty {
            if let Ok(mut guard) = pty.lock() {
                guard.write(bytes);
            }
        }
    }

    pub fn resize(&mut self, rows: u16, cols: u16) {
        let rows = rows.max(1);
        let cols = cols.max(1);
        // Skip if dimensions haven't changed — avoids SIGWINCH spam every frame
        if self.last_size == Some((rows, cols)) {
            return;
        }
        self.last_size = Some((rows, cols));
        if let Ok(mut parser) = self.parser.lock() {
            parser.set_size(rows, cols);
        }
        if let Some(ref pty) = self.pty {
            if let Ok(guard) = pty.lock() {
                guard.resize(rows, cols);
            }
        }
    }

    pub fn kill(&mut self) {
        if let Some(pty) = self.pty.take() {
            if let Ok(mut guard) = pty.lock() {
                guard.kill();
            }
        }
    }
}

#[derive(Debug, PartialEq, Clone)]
pub enum Tab {
    Harness(HarnessTab),
    File(FileTab),
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
    pub file_tree: FileTree,
    pub launcher_input: Option<String>,
}

impl SplashApp {
    pub fn new(config: HarnessConfig) -> Self {
        let file_tree = FileTree::new(".").unwrap_or_else(|_| FileTree::new("/").unwrap());
        Self::with_file_tree(config, file_tree)
    }

    pub fn with_file_tree(config: HarnessConfig, file_tree: FileTree) -> Self {
        let initial_tab = Tab::Harness(HarnessTab::new(config.command.clone()));
        Self {
            config,
            leader_state: LeaderState::default(),
            focus: Focus::MainPane,
            tabs: vec![initial_tab],
            active_tab_index: 0,
            raw_output: String::new(),
            terminal_size: (78, 22),
            parser: Parser::new(22, 78, 1000),
            file_tree,
            launcher_input: None,
        }
    }

    pub fn close_tab(&mut self, index: usize) -> Option<Tab> {
        if index >= self.tabs.len() {
            return None;
        }
        let mut closed = self.tabs.remove(index);
        if let Tab::Harness(ref mut harness_tab) = closed {
            harness_tab.kill();
        }

        if self.tabs.is_empty() {
            self.active_tab_index = 0;
        } else if index <= self.active_tab_index {
            self.active_tab_index = if self.active_tab_index > 0 {
                self.active_tab_index - 1
            } else {
                0
            };
        }
        Some(closed)
    }

    pub fn open_or_focus_file(&mut self, path: impl Into<PathBuf>) -> io::Result<()> {
        let path = path.into();
        if let Some(index) = self.tabs.iter().position(|t| match t {
            Tab::File(f) => f.path == path,
            _ => false,
        }) {
            if let Tab::File(ref mut f) = self.tabs[index] {
                let _ = f.reload();
            }
            self.active_tab_index = index;
        } else {
            let file_tab = FileTab::open(&path)?;
            self.tabs.push(Tab::File(file_tab));
            self.active_tab_index = self.tabs.len() - 1;
        }
        self.focus = Focus::MainPane;
        Ok(())
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
                Tab::Harness(harness_tab) => format!(" [{}: {}] ", i + 1, harness_tab.command),
                Tab::File(file_tab) => {
                    let display_name = file_tab
                        .path
                        .file_name()
                        .map(|n| n.to_string_lossy().to_string())
                        .unwrap_or_else(|| file_tab.path.to_string_lossy().to_string());
                    format!(" [{}: {}] ", i + 1, display_name)
                }
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

        let tree_lines: Vec<Line> = self
            .file_tree
            .entries()
            .iter()
            .enumerate()
            .map(|(i, node)| {
                let indent = "  ".repeat(node.depth);
                let prefix = if node.is_dir {
                    if node.is_expanded {
                        "▼ "
                    } else {
                        "▶ "
                    }
                } else {
                    "  "
                };
                let line_str = format!("{}{}{}", indent, prefix, node.name);
                let style = if i == self.file_tree.selected_index() && self.focus == Focus::FileTree {
                    Style::default().fg(RColor::Yellow).add_modifier(Modifier::BOLD)
                } else {
                    Style::default()
                };
                Line::from(Span::styled(line_str, style))
            })
            .collect();

        let file_tree_paragraph = Paragraph::new(tree_lines).block(tree_block);
        frame.render_widget(file_tree_paragraph, file_tree_area);

        if let Some(ref input) = self.launcher_input {
            let leader_active = self.leader_state.is_active();
            let main_title = if leader_active {
                " Main Pane (Harness Launcher) [LEADER ACTIVE] "
            } else {
                " Main Pane (Harness Launcher) "
            };

            let main_block = Block::default()
                .title(main_title)
                .borders(Borders::ALL)
                .border_style(main_border_style);

            let inner_main_area = main_block.inner(main_pane_area);
            let text = Text::from(vec![
                Line::from(Span::styled("Harness Launcher", Style::default().add_modifier(Modifier::BOLD))),
                Line::from(""),
                Line::from("Enter a harness command to spawn (e.g. agy, claude, bash):"),
                Line::from(""),
                Line::from(vec![
                    Span::styled("> ", Style::default().fg(RColor::Yellow).add_modifier(Modifier::BOLD)),
                    Span::raw(input.as_str()),
                ]),
                Line::from(""),
                Line::from(Span::styled("[Enter] Launch  [Esc] Cancel", Style::default().fg(RColor::DarkGray))),
            ]);
            let paragraph = Paragraph::new(text);

            frame.render_widget(main_block, main_pane_area);
            frame.render_widget(paragraph, inner_main_area);
        } else if self.tabs.is_empty() {
            let leader_active = self.leader_state.is_active();
            let main_title = if leader_active {
                " Main Pane (Empty Workspace) [LEADER ACTIVE] "
            } else {
                " Main Pane (Empty Workspace) "
            };

            let main_block = Block::default()
                .title(main_title)
                .borders(Borders::ALL)
                .border_style(main_border_style);

            let inner_main_area = main_block.inner(main_pane_area);
            let empty_text = Text::from(vec![
                Line::from(Span::styled("Empty Workspace", Style::default().add_modifier(Modifier::BOLD))),
                Line::from(""),
                Line::from("No tabs are currently open."),
                Line::from(""),
                Line::from("• Press 'Ctrl+B h' to launch a harness"),
                Line::from("• Focus the File Tree ('Ctrl+B Left') and press 'Enter' on a file to open"),
            ]);
            let paragraph = Paragraph::new(empty_text);

            frame.render_widget(main_block, main_pane_area);
            frame.render_widget(paragraph, inner_main_area);
        } else {
            let leader_active = self.leader_state.is_active();
            let focus = self.focus;
            let active_tab = &mut self.tabs[self.active_tab_index];
            match active_tab {
                Tab::Harness(harness_tab) => {
                    let main_title = if leader_active {
                        format!(" Main Pane (Harness: {}) [LEADER ACTIVE] ", harness_tab.command)
                    } else {
                        format!(" Main Pane (Harness: {}) ", harness_tab.command)
                    };

                    let main_block = Block::default()
                        .title(main_title)
                        .borders(Borders::ALL)
                        .border_style(main_border_style);

                    let inner_main_area = main_block.inner(main_pane_area);

                    // Resize to exact inner dims from ratatui layout — dirty-tracked so
                    // this only actually resizes (and sends SIGWINCH) when dims change.
                    harness_tab.resize(inner_main_area.height, inner_main_area.width);

                    if let Ok(parser) = harness_tab.parser.lock() {
                        let screen = parser.screen();
                        let text = vt100_screen_to_ratatui_text(screen);
                        // Keep only the last visible rows to fit the inner pane height
                        let total_lines = text.lines.len();
                        let max_visible = inner_main_area.height as usize;
                        let trimmed_text = if total_lines > max_visible {
                            let start = total_lines - max_visible;
                            let trimmed_lines = text.lines[start..].to_vec();
                            Text::from(trimmed_lines)
                        } else {
                            // Use original text directly as it fits
                            text
                        };
                        let main_paragraph = Paragraph::new(trimmed_text);

                        frame.render_widget(main_block, main_pane_area);
                        frame.render_widget(main_paragraph, inner_main_area);

                        if focus == Focus::MainPane && !screen.hide_cursor() {
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
                }
                Tab::File(file_tab) => {
                    let display_name = file_tab
                        .path
                        .file_name()
                        .map(|n| n.to_string_lossy().to_string())
                        .unwrap_or_else(|| file_tab.path.to_string_lossy().to_string());

                    let main_title = if leader_active {
                        format!(" Main Pane (File: {}) [LEADER ACTIVE] ", display_name)
                    } else {
                        format!(" Main Pane (File: {}) ", display_name)
                    };

                    let main_block = Block::default()
                        .title(main_title)
                        .borders(Borders::ALL)
                        .border_style(main_border_style);

                    let inner_main_area = main_block.inner(main_pane_area);

                    let paragraph = Paragraph::new(file_tab.content.as_str())
                        .wrap(Wrap { trim: false })
                        .scroll((file_tab.scroll_offset, 0));

                    frame.render_widget(main_block, main_pane_area);
                    frame.render_widget(paragraph, inner_main_area);
                }
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
            KeyAction::CloseTab => {
                if !self.tabs.is_empty() {
                    self.close_tab(self.active_tab_index);
                }
                KeyAction::None
            }
            KeyAction::OpenLauncher => {
                self.launcher_input = Some(String::new());
                self.focus = Focus::MainPane;
                KeyAction::None
            }
            KeyAction::Forward(bytes) => {
                if let Some(ref mut input) = self.launcher_input {
                    if !self.leader_state.is_active() {
                        match key.code {
                            crossterm::event::KeyCode::Esc => {
                                self.launcher_input = None;
                            }
                            crossterm::event::KeyCode::Backspace => {
                                input.pop();
                            }
                            crossterm::event::KeyCode::Char(c) => {
                                input.push(c);
                            }
                            crossterm::event::KeyCode::Enter => {
                                let input_str = input.trim().to_string();
                                if !input_str.is_empty() {
                                    let parts: Vec<String> = input_str
                                        .split_whitespace()
                                        .map(|s| s.to_string())
                                        .collect();
                                    let cmd = parts[0].clone();
                                    let args = parts[1..].to_vec();
                                    let inner_height = self.terminal_size.1.saturating_sub(3).max(1);
                                    let inner_width = self.terminal_size.0.saturating_sub(2).max(1);

                                    let mut harness_tab = HarnessTab::with_args(cmd, args);
                                    harness_tab.spawn_pty(inner_height, inner_width);

                                    self.tabs.push(Tab::Harness(harness_tab));
                                    self.active_tab_index = self.tabs.len() - 1;
                                    self.focus = Focus::MainPane;
                                }
                                self.launcher_input = None;
                            }
                            _ => {}
                        }
                    }
                    KeyAction::None
                } else if self.tabs.is_empty() {
                    KeyAction::None
                } else if self.focus == Focus::MainPane {
                    if let Tab::File(ref mut file_tab) = self.tabs[self.active_tab_index] {
                        if !self.leader_state.is_active() {
                            let inner_height = self.terminal_size.1.saturating_sub(3).max(1);
                            let half_page = (inner_height / 2).max(1);
                            let max_scroll = file_tab.max_scroll_offset();

                            match key.code {
                                crossterm::event::KeyCode::Up | crossterm::event::KeyCode::Char('k') => {
                                    file_tab.scroll_offset = file_tab.scroll_offset.saturating_sub(1);
                                }
                                crossterm::event::KeyCode::Down | crossterm::event::KeyCode::Char('j') => {
                                    file_tab.scroll_offset =
                                        file_tab.scroll_offset.saturating_add(1).min(max_scroll);
                                }
                                crossterm::event::KeyCode::PageUp => {
                                    file_tab.scroll_offset =
                                        file_tab.scroll_offset.saturating_sub(half_page);
                                }
                                crossterm::event::KeyCode::PageDown => {
                                    file_tab.scroll_offset =
                                        file_tab.scroll_offset.saturating_add(half_page).min(max_scroll);
                                }
                                _ => {}
                            }
                        }
                        KeyAction::None
                    } else if let Tab::Harness(harness_tab) = &self.tabs[self.active_tab_index] {
                        harness_tab.write(&bytes);
                        KeyAction::Forward(bytes)
                    } else {
                        KeyAction::Forward(bytes)
                    }
                } else {
                    if !self.leader_state.is_active() {
                        match key.code {
                            crossterm::event::KeyCode::Up | crossterm::event::KeyCode::Char('k') => {
                                self.file_tree.move_up()
                            }
                            crossterm::event::KeyCode::Down | crossterm::event::KeyCode::Char('j') => {
                                self.file_tree.move_down()
                            }
                            crossterm::event::KeyCode::Right => {
                                self.file_tree.expand_or_select_child()
                            }
                            crossterm::event::KeyCode::Enter => {
                                if let Some(entry) = self.file_tree.selected_entry().cloned() {
                                    if entry.is_dir {
                                        self.file_tree.expand_or_select_child();
                                    } else {
                                        let _ = self.open_or_focus_file(entry.path);
                                    }
                                }
                            }
                            crossterm::event::KeyCode::Left => {
                                self.file_tree.collapse_or_select_parent()
                            }
                            _ => {}
                        }
                    }
                    KeyAction::None
                }
            }
            KeyAction::None => {
                if let Some(ref mut input) = self.launcher_input {
                    if !self.leader_state.is_active() {
                        match key.code {
                            crossterm::event::KeyCode::Esc => {
                                self.launcher_input = None;
                            }
                            crossterm::event::KeyCode::Backspace => {
                                input.pop();
                            }
                            crossterm::event::KeyCode::Char(c) => {
                                input.push(c);
                            }
                            crossterm::event::KeyCode::Enter => {
                                let input_str = input.trim().to_string();
                                if !input_str.is_empty() {
                                    let parts: Vec<String> = input_str
                                        .split_whitespace()
                                        .map(|s| s.to_string())
                                        .collect();
                                    let cmd = parts[0].clone();
                                    let args = parts[1..].to_vec();
                                    let inner_height = self.terminal_size.1.saturating_sub(3).max(1);
                                    let inner_width = self.terminal_size.0.saturating_sub(2).max(1);

                                    let mut harness_tab = HarnessTab::with_args(cmd, args);
                                    harness_tab.spawn_pty(inner_height, inner_width);

                                    self.tabs.push(Tab::Harness(harness_tab));
                                    self.active_tab_index = self.tabs.len() - 1;
                                    self.focus = Focus::MainPane;
                                }
                                self.launcher_input = None;
                            }
                            _ => {}
                        }
                    }
                } else if self.tabs.is_empty() {
                    // Empty workspace: no active tab key routing
                } else if self.focus == Focus::MainPane {
                    if let Tab::File(ref mut file_tab) = self.tabs[self.active_tab_index] {
                        if !self.leader_state.is_active() {
                            let inner_height = self.terminal_size.1.saturating_sub(3).max(1);
                            let half_page = (inner_height / 2).max(1);
                            let max_scroll = file_tab.max_scroll_offset();

                            match key.code {
                                crossterm::event::KeyCode::Up | crossterm::event::KeyCode::Char('k') => {
                                    file_tab.scroll_offset = file_tab.scroll_offset.saturating_sub(1);
                                }
                                crossterm::event::KeyCode::Down | crossterm::event::KeyCode::Char('j') => {
                                    file_tab.scroll_offset =
                                        file_tab.scroll_offset.saturating_add(1).min(max_scroll);
                                }
                                crossterm::event::KeyCode::PageUp => {
                                    file_tab.scroll_offset =
                                        file_tab.scroll_offset.saturating_sub(half_page);
                                }
                                crossterm::event::KeyCode::PageDown => {
                                    file_tab.scroll_offset =
                                        file_tab.scroll_offset.saturating_add(half_page).min(max_scroll);
                                }
                                _ => {}
                            }
                        }
                    }
                } else if self.focus == Focus::FileTree && !self.leader_state.is_active() {
                    match key.code {
                        crossterm::event::KeyCode::Up | crossterm::event::KeyCode::Char('k') => {
                            self.file_tree.move_up()
                        }
                        crossterm::event::KeyCode::Down | crossterm::event::KeyCode::Char('j') => {
                            self.file_tree.move_down()
                        }
                        crossterm::event::KeyCode::Right => {
                            self.file_tree.expand_or_select_child()
                        }
                        crossterm::event::KeyCode::Enter => {
                            if let Some(entry) = self.file_tree.selected_entry().cloned() {
                                if entry.is_dir {
                                    self.file_tree.expand_or_select_child();
                                } else {
                                    let _ = self.open_or_focus_file(entry.path);
                                }
                            }
                        }
                        crossterm::event::KeyCode::Left => {
                            self.file_tree.collapse_or_select_parent()
                        }
                        _ => {}
                    }
                }
                KeyAction::None
            }
        }
    }

    pub fn tick(&mut self) {
        for tab in &mut self.tabs {
            if let Tab::Harness(harness_tab) = tab {
                harness_tab.tick();
            }
        }
    }

    pub fn push_output_chunk(&mut self, text: &str) {
        self.raw_output.push_str(text);
        self.parser.process(text.as_bytes());
        if let Some(Tab::Harness(harness_tab)) = self.tabs.get_mut(self.active_tab_index) {
            if let Ok(mut parser) = harness_tab.parser.lock() {
                parser.process(text.as_bytes());
            }
        }
    }

    pub fn set_size(&mut self, width: u16, height: u16) {
        self.terminal_size = (width, height);
        self.parser.set_size(height.max(1), width.max(1));
        // Mirror ratatui layout: 1 row tab bar, pane borders top+bottom=2 rows
        let inner_height = height.saturating_sub(3).max(1);
        // Main pane gets 80% of width; then subtract 2 for its own borders.
        // Ratatui uses integer (width * 80 / 100) for Percentage(80).
        let main_pane_width = width * 80 / 100;
        let inner_width = main_pane_width.saturating_sub(2).max(1);
        for tab in &mut self.tabs {
            if let Tab::Harness(harness_tab) = tab {
                harness_tab.resize(inner_height, inner_width);
            }
        }
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
                // vt100 represents blank/never-written cells as contents() == "".
                // These are visually empty but must render as spaces so that
                // words separated by cursor-movement gaps don't collide ("Ican" bug).
                let symbol = if symbol.is_empty() { " ".to_string() } else { symbol };

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
    use crate::testing::format_buffer_grid;

    #[test]
    fn test_vt100_space_between_style_runs_preserved() {
        // The vt100 crate represents blank/space cells as cell.contents() == "".
        // When a space sits between two differently-styled runs (e.g. bold "I" then
        // normal " can"), the style-change flush must still emit the space — otherwise
        // words run together: "I can" becomes "Ican".
        let mut parser = vt100::Parser::new(3, 30, 0);
        // Bold "I", reset to normal, then " can" — the space is in the normal-style region.
        parser.process(b"\x1b[1mI\x1b[m can");
        let screen = parser.screen();
        let text = vt100_screen_to_ratatui_text(screen);

        let row_text: String = text.lines[0]
            .spans
            .iter()
            .map(|s| s.content.as_ref())
            .collect();

        assert!(
            row_text.starts_with("I can"),
            "Space between styled runs was swallowed; row text: {:?}",
            row_text
        );
    }

    #[test]
    fn test_vt100_plain_spaces_preserved() {
        // Plain text with multiple spaces — every space must render, not be silently dropped.
        let mut parser = vt100::Parser::new(3, 30, 0);
        parser.process(b"hello world foo bar");
        let screen = parser.screen();
        let text = vt100_screen_to_ratatui_text(screen);

        let row_text: String = text.lines[0]
            .spans
            .iter()
            .map(|s| s.content.as_ref())
            .collect();

        assert!(
            row_text.contains("hello world foo bar"),
            "Plain spaces were dropped in rendering; got: {:?}",
            row_text
        );
    }

    #[test]
    fn test_vt100_cursor_skipped_cells_render_as_spaces() {
        // TUI apps (like agy) often use cursor-forward (ESC[nC) to position text
        // rather than writing explicit spaces. Cells skipped this way are never
        // written and have cell.contents() == "". They must still render as spaces
        // or words/elements visually collide.
        let mut parser = vt100::Parser::new(3, 20, 0);
        // Write "a", then jump cursor forward 2 columns, then write "b".
        // Columns 1 and 2 are never written to.
        parser.process(b"a\x1b[2Cb");  // ESC[2C = cursor forward 2

        let screen = parser.screen();
        let text = vt100_screen_to_ratatui_text(screen);

        let row_text: String = text.lines[0]
            .spans
            .iter()
            .map(|s| s.content.as_ref())
            .collect();

        // The two never-written cells must appear as spaces so "a" and "b" stay separated.
        assert!(
            row_text.starts_with("a  b"),
            "Never-written cells (cursor-skipped) were not rendered as spaces; got: {:?}",
            row_text
        );
    }

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
        app.tabs.push(Tab::File(FileTab {
            path: PathBuf::from("main.rs"),
            content: "fn main() {}".to_string(),
            scroll_offset: 0,
        }));

        let key_ctrl_b = KeyEvent::new(KeyCode::Char('b'), crossterm::event::KeyModifiers::CONTROL);
        let key_2 = KeyEvent::new(KeyCode::Char('2'), crossterm::event::KeyModifiers::empty());
        app.handle_key_event(&key_ctrl_b);
        app.handle_key_event(&key_2);

        assert_eq!(app.active_tab_index, 1);
    }

    #[test]
    fn test_file_tree_keyboard_navigation_routing() {
        let temp_dir = std::env::temp_dir().join(format!("splash_test_app_tree_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&temp_dir);
        std::fs::create_dir_all(temp_dir.join("dir1")).unwrap();
        std::fs::File::create(temp_dir.join("file1.txt")).unwrap();
        std::fs::File::create(temp_dir.join("file2.txt")).unwrap();

        let file_tree = FileTree::new(&temp_dir).unwrap();
        let config = HarnessConfig {
            command: "bash".to_string(),
            args: vec![],
        };
        let mut app = SplashApp::with_file_tree(config, file_tree);
        app.focus = Focus::FileTree;

        // Initially selected_index is 0 ("dir1")
        assert_eq!(app.file_tree.selected_index(), 0);

        // Down arrow / 'j' key -> move_down
        let key_down = KeyEvent::new(KeyCode::Down, crossterm::event::KeyModifiers::empty());
        assert_eq!(app.handle_key_event(&key_down), KeyAction::None);
        assert_eq!(app.file_tree.selected_index(), 1);

        let key_j = KeyEvent::new(KeyCode::Char('j'), crossterm::event::KeyModifiers::empty());
        assert_eq!(app.handle_key_event(&key_j), KeyAction::None);
        assert_eq!(app.file_tree.selected_index(), 2);

        // Up arrow / 'k' key -> move_up
        let key_k = KeyEvent::new(KeyCode::Char('k'), crossterm::event::KeyModifiers::empty());
        assert_eq!(app.handle_key_event(&key_k), KeyAction::None);
        assert_eq!(app.file_tree.selected_index(), 1);

        let key_up = KeyEvent::new(KeyCode::Up, crossterm::event::KeyModifiers::empty());
        assert_eq!(app.handle_key_event(&key_up), KeyAction::None);
        assert_eq!(app.file_tree.selected_index(), 0);

        // Right arrow / Enter on "dir1" (index 0) -> expand_or_select_child
        let key_right = KeyEvent::new(KeyCode::Right, crossterm::event::KeyModifiers::empty());
        assert_eq!(app.handle_key_event(&key_right), KeyAction::None);
        assert_eq!(app.file_tree.entries()[0].is_expanded, true);

        // Left arrow -> collapse_or_select_parent
        let key_left = KeyEvent::new(KeyCode::Left, crossterm::event::KeyModifiers::empty());
        assert_eq!(app.handle_key_event(&key_left), KeyAction::None);
        assert_eq!(app.file_tree.entries()[0].is_expanded, false);

        let _ = std::fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_file_tree_rendering_with_icons_and_highlighting() {
        use ratatui::backend::TestBackend;
        use ratatui::Terminal;

        let temp_dir = std::env::temp_dir().join(format!("splash_test_app_render_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&temp_dir);
        std::fs::create_dir_all(temp_dir.join("sub")).unwrap();
        std::fs::File::create(temp_dir.join("root_file.txt")).unwrap();
        std::fs::File::create(temp_dir.join("sub/nested.txt")).unwrap();

        let mut file_tree = FileTree::new(&temp_dir).unwrap();
        // Expand "sub" (index 0)
        file_tree.expand_or_select_child();

        let config = HarnessConfig {
            command: "bash".to_string(),
            args: vec![],
        };
        let mut app = SplashApp::with_file_tree(config, file_tree);
        app.focus = Focus::FileTree;

        let backend = TestBackend::new(80, 24);
        let mut terminal = Terminal::new(backend).unwrap();

        terminal.draw(|f| app.render(f)).unwrap();

        let buffer = terminal.backend().buffer();
        // Content inside tree block starts at x=1, y=2 (y=1 is top border)
        let cell_row1 = buffer.get(1, 2);
        assert_eq!(cell_row1.symbol(), "▼");
        assert_eq!(cell_row1.fg, RColor::Yellow); // highlighted because index 0 selected and FileTree focused

        // Row 2 at y=3: "    nested.txt"
        let cell_row2_indent = buffer.get(1, 3);
        assert_eq!(cell_row2_indent.symbol(), " ");
        let cell_row2_char = buffer.get(5, 3); // 4 spaces indent + 'n'
        assert_eq!(cell_row2_char.symbol(), "n");

        let _ = std::fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_file_tab_open_lossy_decoding_and_reload() {
        let temp_dir = std::env::temp_dir().join(format!("splash_test_file_tab_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&temp_dir);
        std::fs::create_dir_all(&temp_dir).unwrap();

        let file_path = temp_dir.join("non_utf8.txt");
        let invalid_utf8_bytes = vec![0xFF, 0xFE, b'H', b'e', b'l', b'l', b'o'];
        std::fs::write(&file_path, &invalid_utf8_bytes).unwrap();

        let mut tab = FileTab::open(&file_path).unwrap();
        assert!(tab.content.contains("Hello"));
        // Lossy UTF-8 decoding replaces invalid bytes with replacement char U+FFFD ()
        assert!(tab.content.contains('\u{FFFD}'));

        // Modify file on disk and reload
        std::fs::write(&file_path, "Updated Content").unwrap();
        tab.reload().unwrap();
        assert_eq!(tab.content, "Updated Content");

        let _ = std::fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_file_tree_enter_opens_and_focuses_file_tab() {
        let temp_dir = std::env::temp_dir().join(format!("splash_test_enter_open_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&temp_dir);
        std::fs::create_dir_all(&temp_dir).unwrap();

        let file_path = temp_dir.join("sample.txt");
        std::fs::write(&file_path, "Hello from sample file").unwrap();

        let file_tree = FileTree::new(&temp_dir).unwrap();
        let config = HarnessConfig {
            command: "bash".to_string(),
            args: vec![],
        };
        let mut app = SplashApp::with_file_tree(config, file_tree);
        app.focus = Focus::FileTree;

        // Selected index 0 is "sample.txt"
        assert_eq!(app.file_tree.selected_entry().unwrap().name, "sample.txt");

        // Press Enter
        let key_enter = KeyEvent::new(KeyCode::Enter, crossterm::event::KeyModifiers::empty());
        app.handle_key_event(&key_enter);

        // Should open new FileTab, focus MainPane, set active tab index to 1
        assert_eq!(app.focus, Focus::MainPane);
        assert_eq!(app.tabs.len(), 2);
        assert_eq!(app.active_tab_index, 1);

        if let Tab::File(ref file_tab) = app.tabs[1] {
            assert_eq!(file_tab.content, "Hello from sample file");
        } else {
            panic!("Expected Tab::File at index 1");
        }

        let _ = std::fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_file_tree_enter_on_already_open_file_tab_reloads_and_focuses() {
        let temp_dir = std::env::temp_dir().join(format!("splash_test_enter_reload_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&temp_dir);
        std::fs::create_dir_all(&temp_dir).unwrap();

        let file_path = temp_dir.join("reload_me.txt");
        std::fs::write(&file_path, "Initial Content").unwrap();

        let file_tree = FileTree::new(&temp_dir).unwrap();
        let config = HarnessConfig {
            command: "bash".to_string(),
            args: vec![],
        };
        let mut app = SplashApp::with_file_tree(config, file_tree);

        // Open initial tab
        app.focus = Focus::FileTree;
        let key_enter = KeyEvent::new(KeyCode::Enter, crossterm::event::KeyModifiers::empty());
        app.handle_key_event(&key_enter);
        assert_eq!(app.tabs.len(), 2);
        assert_eq!(app.active_tab_index, 1);

        // Switch active tab back to Harness tab (index 0)
        app.active_tab_index = 0;

        // Modify file on disk
        std::fs::write(&file_path, "Disk Content Has Changed").unwrap();

        // Focus FileTree and press Enter on "reload_me.txt" again
        app.focus = Focus::FileTree;
        app.handle_key_event(&key_enter);

        // Should NOT open a 3rd tab, should focus index 1, and reload content
        assert_eq!(app.tabs.len(), 2);
        assert_eq!(app.active_tab_index, 1);
        assert_eq!(app.focus, Focus::MainPane);

        if let Tab::File(ref file_tab) = app.tabs[1] {
            assert_eq!(file_tab.content, "Disk Content Has Changed");
        } else {
            panic!("Expected Tab::File at index 1");
        }

        let _ = std::fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_main_pane_rendering_file_tab_with_wrapping() {
        use ratatui::backend::TestBackend;
        use ratatui::Terminal;

        let config = HarnessConfig {
            command: "bash".to_string(),
            args: vec![],
        };
        let mut app = SplashApp::new(config);
        let long_line = "This is a very long line of text that should wrap across multiple lines when rendered inside the MainPane of SplashApp.";
        app.tabs.push(Tab::File(FileTab {
            path: PathBuf::from("long.txt"),
            content: long_line.to_string(),
            scroll_offset: 0,
        }));
        app.active_tab_index = 1;

        let backend = TestBackend::new(40, 10);
        let mut terminal = Terminal::new(backend).unwrap();

        terminal.draw(|f| app.render(f)).unwrap();

        let buffer = terminal.backend().buffer();

        // Check title in main block
        let mut title_found = false;
        for x in 0..40 {
            let symbol = buffer.get(x, 1).symbol();
            if symbol == "l" || symbol == "l" {
                title_found = true;
                break;
            }
        }
        assert!(title_found || buffer.get(0, 1).symbol() != "");

        // Verify content wraps: line 1 inside main block (y=2) and line 2 (y=3) contain parts of long_line
        let row1_str: String = (1..39).map(|x| buffer.get(x, 2).symbol()).collect();
        let row2_str: String = (1..39).map(|x| buffer.get(x, 3).symbol()).collect();

        assert!(row1_str.contains("This is a very long line"));
        assert!(row2_str.contains("wrap across"));
    }

    #[test]
    fn test_file_tab_line_by_line_scrolling_and_clamping() {
        let config = HarnessConfig {
            command: "bash".to_string(),
            args: vec![],
        };
        let mut app = SplashApp::new(config);
        let content = (1..=10).map(|i| format!("Line {}", i)).collect::<Vec<_>>().join("\n");
        app.tabs.push(Tab::File(FileTab {
            path: PathBuf::from("lines.txt"),
            content,
            scroll_offset: 0,
        }));
        app.active_tab_index = 1;
        app.focus = Focus::MainPane;

        let key_down = KeyEvent::new(KeyCode::Down, crossterm::event::KeyModifiers::empty());
        let key_up = KeyEvent::new(KeyCode::Up, crossterm::event::KeyModifiers::empty());

        // Initially scroll_offset is 0
        if let Tab::File(ref f) = app.tabs[1] {
            assert_eq!(f.scroll_offset, 0);
        }

        // Press Down key: scroll_offset becomes 1
        app.handle_key_event(&key_down);
        if let Tab::File(ref f) = app.tabs[1] {
            assert_eq!(f.scroll_offset, 1);
        } else {
            panic!("Expected Tab::File");
        }

        // Press Up key: scroll_offset becomes 0
        app.handle_key_event(&key_up);
        if let Tab::File(ref f) = app.tabs[1] {
            assert_eq!(f.scroll_offset, 0);
        }

        // Press Up key at 0: remains 0 (clamped)
        app.handle_key_event(&key_up);
        if let Tab::File(ref f) = app.tabs[1] {
            assert_eq!(f.scroll_offset, 0);
        }

        // Press Down key 20 times: max lines is 10 (indices 0..9), so max_scroll_offset is 9
        for _ in 0..20 {
            app.handle_key_event(&key_down);
        }
        if let Tab::File(ref f) = app.tabs[1] {
            assert_eq!(f.scroll_offset, 9);
        }
    }

    #[test]
    fn test_file_tab_page_scrolling_and_clamping() {
        let config = HarnessConfig {
            command: "bash".to_string(),
            args: vec![],
        };
        let mut app = SplashApp::new(config);
        // Terminal size is 78x22 -> inner main height is 22 - 1 (tab bar) - 2 (borders) = 19
        // half-screen height is 19 / 2 = 9
        let content = (1..=100).map(|i| format!("Line {}", i)).collect::<Vec<_>>().join("\n");
        app.tabs.push(Tab::File(FileTab {
            path: PathBuf::from("big.txt"),
            content,
            scroll_offset: 0,
        }));
        app.active_tab_index = 1;
        app.focus = Focus::MainPane;

        let key_pgdn = KeyEvent::new(KeyCode::PageDown, crossterm::event::KeyModifiers::empty());
        let key_pgup = KeyEvent::new(KeyCode::PageUp, crossterm::event::KeyModifiers::empty());

        // Press PageDown: scroll_offset increases by half-screen height (9)
        app.handle_key_event(&key_pgdn);
        if let Tab::File(ref f) = app.tabs[1] {
            assert_eq!(f.scroll_offset, 9);
        } else {
            panic!("Expected Tab::File");
        }

        // Press PageUp: scroll_offset decreases by 9 back to 0
        app.handle_key_event(&key_pgup);
        if let Tab::File(ref f) = app.tabs[1] {
            assert_eq!(f.scroll_offset, 0);
        }

        // Press PageDown 20 times: clamps at max_scroll_offset = 99
        for _ in 0..20 {
            app.handle_key_event(&key_pgdn);
        }
        if let Tab::File(ref f) = app.tabs[1] {
            assert_eq!(f.scroll_offset, 99);
        }
    }

    #[test]
    fn test_file_tab_scrolled_rendering() {
        use ratatui::backend::TestBackend;
        use ratatui::Terminal;

        let config = HarnessConfig {
            command: "bash".to_string(),
            args: vec![],
        };
        let mut app = SplashApp::new(config);
        let content = "First Line\nSecond Line\nThird Line\nFourth Line\nFifth Line";
        app.tabs.push(Tab::File(FileTab {
            path: PathBuf::from("scroll_render.txt"),
            content: content.to_string(),
            scroll_offset: 2,
        }));
        app.active_tab_index = 1;

        let backend = TestBackend::new(40, 10);
        let mut terminal = Terminal::new(backend).unwrap();

        terminal.draw(|f| app.render(f)).unwrap();

        let buffer = terminal.backend().buffer();
        // Inner area starts at y=2. When scroll_offset=2, row y=2 should display "Third Line"
        let row1_str: String = (1..39).map(|x| buffer.get(x, 2).symbol()).collect();
        assert!(row1_str.contains("Third Line"));
        assert!(!row1_str.contains("First Line"));
    }

    #[test]
    fn test_close_tab_indexing_and_empty_workspace_transition() {
        let config = HarnessConfig {
            command: "bash".to_string(),
            args: vec![],
        };
        let mut app = SplashApp::new(config);
        app.tabs.push(Tab::File(FileTab {
            path: PathBuf::from("a.txt"),
            content: "a".to_string(),
            scroll_offset: 0,
        }));
        app.tabs.push(Tab::File(FileTab {
            path: PathBuf::from("b.txt"),
            content: "b".to_string(),
            scroll_offset: 0,
        }));
        app.active_tab_index = 2; // "b.txt"

        let key_ctrl_b = KeyEvent::new(KeyCode::Char('b'), crossterm::event::KeyModifiers::CONTROL);
        let key_w = KeyEvent::new(KeyCode::Char('w'), crossterm::event::KeyModifiers::empty());

        // Close active tab 2 ("b.txt") -> active_tab_index becomes 1 ("a.txt")
        app.handle_key_event(&key_ctrl_b);
        assert_eq!(app.handle_key_event(&key_w), KeyAction::None);
        assert_eq!(app.tabs.len(), 2);
        assert_eq!(app.active_tab_index, 1);

        // Close active tab 1 ("a.txt") -> active_tab_index becomes 0 ("bash")
        app.handle_key_event(&key_ctrl_b);
        app.handle_key_event(&key_w);
        assert_eq!(app.tabs.len(), 1);
        assert_eq!(app.active_tab_index, 0);

        // Close active tab 0 ("bash") -> tabs becomes empty, active_tab_index stays 0
        app.handle_key_event(&key_ctrl_b);
        app.handle_key_event(&key_w);
        assert!(app.tabs.is_empty());
        assert_eq!(app.active_tab_index, 0);
    }

    #[test]
    fn test_close_harness_tab_kills_pty() {
        let config = HarnessConfig {
            command: "echo".to_string(),
            args: vec!["test".to_string()],
        };
        let pty = PtyHarness::spawn(&config, 24, 80).unwrap();
        let harness_tab = HarnessTab::with_pty("echo", pty, 24, 80);

        let mut app = SplashApp::new(config);
        app.tabs[0] = Tab::Harness(harness_tab);

        // Closing harness tab 0 triggers kill() on underlying pty
        let closed = app.close_tab(0);
        assert!(closed.is_some());
        assert!(app.tabs.is_empty());
    }

    #[test]
    fn test_empty_workspace_rendering_and_leader_navigation() {
        use ratatui::backend::TestBackend;
        use ratatui::Terminal;

        let config = HarnessConfig {
            command: "bash".to_string(),
            args: vec![],
        };
        let mut app = SplashApp::new(config);
        app.tabs.clear();
        assert!(app.tabs.is_empty());

        // Render Empty Workspace
        let backend = TestBackend::new(80, 24);
        let mut terminal = Terminal::new(backend).unwrap();

        terminal.draw(|f| app.render(f)).unwrap();
        let buffer = terminal.backend().buffer();

        let grid = format_buffer_grid(buffer);
        assert!(grid.contains("Empty Workspace"));
        assert!(grid.contains("Ctrl+B h"));

        // Leader shortcuts work in Empty Workspace
        let key_ctrl_b = KeyEvent::new(KeyCode::Char('b'), crossterm::event::KeyModifiers::CONTROL);
        let key_left = KeyEvent::new(KeyCode::Left, crossterm::event::KeyModifiers::empty());
        let key_right = KeyEvent::new(KeyCode::Right, crossterm::event::KeyModifiers::empty());

        app.handle_key_event(&key_ctrl_b);
        app.handle_key_event(&key_left);
        assert_eq!(app.focus, Focus::FileTree);

        app.handle_key_event(&key_ctrl_b);
        app.handle_key_event(&key_right);
        assert_eq!(app.focus, Focus::MainPane);
    }

    #[test]
    fn test_launcher_activation_and_esc_cancel() {
        let config = HarnessConfig {
            command: "bash".to_string(),
            args: vec![],
        };
        let mut app = SplashApp::new(config);
        assert!(app.launcher_input.is_none());

        let key_ctrl_b = KeyEvent::new(KeyCode::Char('b'), crossterm::event::KeyModifiers::CONTROL);
        let key_h = KeyEvent::new(KeyCode::Char('h'), crossterm::event::KeyModifiers::empty());
        let key_esc = KeyEvent::new(KeyCode::Esc, crossterm::event::KeyModifiers::empty());

        // Press Ctrl+B h -> opens launcher prompt
        app.handle_key_event(&key_ctrl_b);
        app.handle_key_event(&key_h);
        assert_eq!(app.launcher_input, Some(String::new()));
        assert_eq!(app.focus, Focus::MainPane);

        // Press Esc -> cancels launcher prompt
        app.handle_key_event(&key_esc);
        assert!(app.launcher_input.is_none());
    }

    #[test]
    fn test_launcher_typing_and_enter_spawns_harness_tab() {
        let config = HarnessConfig {
            command: "bash".to_string(),
            args: vec![],
        };
        let mut app = SplashApp::new(config);

        // Open launcher
        let key_ctrl_b = KeyEvent::new(KeyCode::Char('b'), crossterm::event::KeyModifiers::CONTROL);
        let key_h = KeyEvent::new(KeyCode::Char('h'), crossterm::event::KeyModifiers::empty());
        app.handle_key_event(&key_ctrl_b);
        app.handle_key_event(&key_h);

        // Type "agy"
        app.handle_key_event(&KeyEvent::new(KeyCode::Char('a'), crossterm::event::KeyModifiers::empty()));
        app.handle_key_event(&KeyEvent::new(KeyCode::Char('g'), crossterm::event::KeyModifiers::empty()));
        app.handle_key_event(&KeyEvent::new(KeyCode::Char('y'), crossterm::event::KeyModifiers::empty()));

        assert_eq!(app.launcher_input, Some("agy".to_string()));

        // Backspace then type "y" again
        app.handle_key_event(&KeyEvent::new(KeyCode::Backspace, crossterm::event::KeyModifiers::empty()));
        assert_eq!(app.launcher_input, Some("ag".to_string()));
        app.handle_key_event(&KeyEvent::new(KeyCode::Char('y'), crossterm::event::KeyModifiers::empty()));

        // Press Enter -> spawns new HarnessTab ("agy"), active index becomes 1, launcher_input becomes None
        app.handle_key_event(&KeyEvent::new(KeyCode::Enter, crossterm::event::KeyModifiers::empty()));

        assert!(app.launcher_input.is_none());
        assert_eq!(app.tabs.len(), 2);
        assert_eq!(app.active_tab_index, 1);
        if let Tab::Harness(ref h) = app.tabs[1] {
            assert_eq!(h.command, "agy");
        } else {
            panic!("Expected Tab::Harness at index 1");
        }
    }

    #[test]
    fn test_launcher_prompt_rendering() {
        use ratatui::backend::TestBackend;
        use ratatui::Terminal;

        let config = HarnessConfig {
            command: "bash".to_string(),
            args: vec![],
        };
        let mut app = SplashApp::new(config);
        app.launcher_input = Some("claude".to_string());

        let backend = TestBackend::new(80, 24);
        let mut terminal = Terminal::new(backend).unwrap();

        terminal.draw(|f| app.render(f)).unwrap();
        let buffer = terminal.backend().buffer();

        let grid = format_buffer_grid(buffer);
        assert!(grid.contains("Harness Launcher"));
        assert!(grid.contains("> claude"));
    }
}


