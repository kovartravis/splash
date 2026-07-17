use std::env;
use std::io::{self, stdout, Read, Write};
use std::sync::mpsc::{channel, Receiver};
use std::thread;
use std::time::Duration;

use crossterm::{
    event::{self, Event, KeyCode, KeyEvent, KeyModifiers},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use portable_pty::{native_pty_system, CommandBuilder, PtyPair, PtySize};
use ratatui::{
    backend::CrosstermBackend,
    widgets::{Block, Borders, Paragraph},
    Terminal,
};

#[derive(Debug, PartialEq, Clone)]
pub struct HarnessConfig {
    pub command: String,
    pub args: Vec<String>,
}

pub fn parse_args(args: &[String]) -> Result<HarnessConfig, String> {
    if args.len() < 2 {
        return Err("Usage: splash <harness-command>".to_string());
    }
    Ok(HarnessConfig {
        command: args[1].clone(),
        args: args[2..].to_vec(),
    })
}

#[derive(Debug, PartialEq, Clone, Default)]
pub enum LeaderState {
    #[default]
    Normal,
    LeaderPressed,
}

#[derive(Debug, PartialEq, Clone)]
pub enum KeyAction {
    None,
    Quit,
    Forward(Vec<u8>),
}

impl LeaderState {
    pub fn is_active(&self) -> bool {
        matches!(self, LeaderState::LeaderPressed)
    }

    pub fn handle_key(&mut self, key: &KeyEvent) -> KeyAction {
        match self {
            LeaderState::Normal => {
                if key.code == KeyCode::Char('b') && key.modifiers.contains(KeyModifiers::CONTROL) {
                    *self = LeaderState::LeaderPressed;
                    KeyAction::None
                } else {
                    let bytes = key_event_to_bytes(key);
                    if bytes.is_empty() {
                        KeyAction::None
                    } else {
                        KeyAction::Forward(bytes)
                    }
                }
            }
            LeaderState::LeaderPressed => {
                *self = LeaderState::Normal;
                match key.code {
                    KeyCode::Char('q') | KeyCode::Char('Q') => KeyAction::Quit,
                    KeyCode::Char('b') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                        KeyAction::Forward(vec![0x02])
                    }
                    _ => KeyAction::None,
                }
            }
        }
    }
}

pub fn key_event_to_bytes(key: &KeyEvent) -> Vec<u8> {
    if key.modifiers.contains(KeyModifiers::CONTROL) {
        match key.code {
            KeyCode::Char(c) => {
                let lower = c.to_ascii_lowercase();
                if lower >= 'a' && lower <= 'z' {
                    vec![(lower as u8) - b'a' + 1]
                } else {
                    vec![]
                }
            }
            _ => vec![],
        }
    } else {
        match key.code {
            KeyCode::Char(c) => {
                let mut buf = [0; 4];
                c.encode_utf8(&mut buf).as_bytes().to_vec()
            }
            KeyCode::Enter => vec![b'\r'],
            KeyCode::Backspace => vec![0x7f],
            KeyCode::Tab => vec![b'\t'],
            KeyCode::Esc => vec![0x1b],
            _ => vec![],
        }
    }
}

pub struct PtyHarness {
    pub pty_pair: PtyPair,
    pub writer: Box<dyn Write + Send>,
    pub output_rx: Receiver<String>,
    #[allow(dead_code)]
    pub child: Box<dyn portable_pty::Child + Send + Sync>,
}

impl PtyHarness {
    pub fn spawn(config: &HarnessConfig, rows: u16, cols: u16) -> Result<Self, String> {
        let pty_system = native_pty_system();
        let pty_pair = pty_system
            .openpty(PtySize {
                rows,
                cols,
                pixel_width: 0,
                pixel_height: 0,
            })
            .map_err(|e| format!("Failed to open PTY: {}", e))?;

        let mut cmd = CommandBuilder::new(&config.command);
        cmd.args(&config.args);
        if let Ok(cwd) = env::current_dir() {
            cmd.cwd(cwd);
        }

        let child = pty_pair
            .slave
            .spawn_command(cmd)
            .map_err(|e| format!("Failed to spawn command '{}': {}", config.command, e))?;

        let writer = pty_pair
            .master
            .take_writer()
            .map_err(|e| format!("Failed to take PTY writer: {}", e))?;

        let mut reader = pty_pair
            .master
            .try_clone_reader()
            .map_err(|e| format!("Failed to clone PTY reader: {}", e))?;

        let (tx, rx) = channel();

        thread::spawn(move || {
            let mut buf = [0u8; 1024];
            loop {
                match reader.read(&mut buf) {
                    Ok(0) => break,
                    Ok(n) => {
                        let text = String::from_utf8_lossy(&buf[..n]).to_string();
                        if tx.send(text).is_err() {
                            break;
                        }
                    }
                    Err(_) => break,
                }
            }
        });

        Ok(PtyHarness {
            pty_pair,
            writer,
            output_rx: rx,
            child,
        })
    }

    pub fn resize(&self, rows: u16, cols: u16) {
        let _ = self.pty_pair.master.resize(PtySize {
            rows,
            cols,
            pixel_width: 0,
            pixel_height: 0,
        });
    }

    pub fn write(&mut self, bytes: &[u8]) {
        let _ = self.writer.write_all(bytes);
        let _ = self.writer.flush();
    }
}

fn main() {
    let args: Vec<String> = env::args().collect();
    let config = match parse_args(&args) {
        Ok(cfg) => cfg,
        Err(err) => {
            eprintln!("{}", err);
            std::process::exit(1);
        }
    };

    // Set up panic hook to restore terminal if we panic
    let default_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        let _ = restore_terminal();
        default_hook(info);
    }));

    if let Err(e) = run_splash(config) {
        let _ = restore_terminal();
        eprintln!("Splash error: {}", e);
        std::process::exit(1);
    }
}

fn restore_terminal() -> io::Result<()> {
    disable_raw_mode()?;
    execute!(stdout(), LeaveAlternateScreen)?;
    Ok(())
}

fn run_splash(config: HarnessConfig) -> Result<(), String> {
    enable_raw_mode().map_err(|e| e.to_string())?;
    let mut stdout = stdout();
    execute!(stdout, EnterAlternateScreen).map_err(|e| e.to_string())?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend).map_err(|e| e.to_string())?;

    let size = terminal.size().map_err(|e| e.to_string())?;
    let mut harness = PtyHarness::spawn(&config, size.height, size.width)?;

    let mut raw_output = String::new();
    let mut leader_state = LeaderState::default();

    loop {
        // Drain incoming PTY output
        while let Ok(chunk) = harness.output_rx.try_recv() {
            raw_output.push_str(&chunk);
        }

        // Keep raw_output bounded to reasonable size if needed (e.g., last 1000 lines)
        let lines: Vec<&str> = raw_output.lines().collect();
        let display_text = if lines.len() > 100 {
            lines[lines.len() - 100..].join("\n")
        } else {
            raw_output.clone()
        };

        let leader_active = leader_state.is_active();
        let cmd_title = format!(" Harness: {} (Leader: Ctrl+B | Exit: Ctrl+B q) ", config.command);

        terminal
            .draw(|f| {
                let rect = f.size();
                harness.resize(rect.height, rect.width);

                let title = if leader_active {
                    format!("{} [LEADER ACTIVE]", cmd_title)
                } else {
                    cmd_title.clone()
                };

                let block = Block::default().title(title).borders(Borders::ALL);
                let paragraph = Paragraph::new(display_text).block(block);
                f.render_widget(paragraph, rect);
            })
            .map_err(|e| e.to_string())?;

        if event::poll(Duration::from_millis(30)).map_err(|e| e.to_string())? {
            if let Event::Key(key) = event::read().map_err(|e| e.to_string())? {
                match leader_state.handle_key(&key) {
                    KeyAction::Quit => break,
                    KeyAction::Forward(bytes) => {
                        harness.write(&bytes);
                    }
                    KeyAction::None => {}
                }
            }
        }
    }

    restore_terminal().map_err(|e| e.to_string())?;
    Ok(())
}

fn should_exit(key: &KeyEvent) -> bool {
    match key.code {
        KeyCode::Char('q') | KeyCode::Char('Q') => true,
        KeyCode::Char('c') | KeyCode::Char('C') => key.modifiers.contains(KeyModifiers::CONTROL),
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_should_exit() {
        let key_q = KeyEvent::new(KeyCode::Char('q'), KeyModifiers::empty());
        let key_cap_q = KeyEvent::new(KeyCode::Char('Q'), KeyModifiers::empty());
        let key_ctrl_c = KeyEvent::new(KeyCode::Char('c'), KeyModifiers::CONTROL);
        let key_cap_ctrl_c = KeyEvent::new(KeyCode::Char('C'), KeyModifiers::CONTROL);
        let key_c = KeyEvent::new(KeyCode::Char('c'), KeyModifiers::empty());
        let key_other = KeyEvent::new(KeyCode::Char('x'), KeyModifiers::empty());

        assert!(should_exit(&key_q));
        assert!(should_exit(&key_cap_q));
        assert!(should_exit(&key_ctrl_c));
        assert!(should_exit(&key_cap_ctrl_c));
        assert!(!should_exit(&key_c));
        assert!(!should_exit(&key_other));
    }

    #[test]
    fn test_parse_args_valid() {
        let args = vec!["splash".to_string(), "bash".to_string()];
        let config = parse_args(&args).unwrap();
        assert_eq!(config.command, "bash");
        assert!(config.args.is_empty());
    }

    #[test]
    fn test_parse_args_with_cmd_args() {
        let args = vec![
            "splash".to_string(),
            "echo".to_string(),
            "hello".to_string(),
            "world".to_string(),
        ];
        let config = parse_args(&args).unwrap();
        assert_eq!(config.command, "echo");
        assert_eq!(config.args, vec!["hello", "world"]);
    }

    #[test]
    fn test_parse_args_missing() {
        let args = vec!["splash".to_string()];
        let err = parse_args(&args).unwrap_err();
        assert!(err.contains("Usage: splash <harness-command>"));
    }

    #[test]
    fn test_leader_state_machine() {
        let mut leader = LeaderState::default();

        // Normal key -> Forward to PTY
        let key_a = KeyEvent::new(KeyCode::Char('a'), KeyModifiers::empty());
        assert_eq!(leader.handle_key(&key_a), KeyAction::Forward(vec![b'a']));

        // Ctrl+B -> Enter LeaderPressed state
        let key_ctrl_b = KeyEvent::new(KeyCode::Char('b'), KeyModifiers::CONTROL);
        assert_eq!(leader.handle_key(&key_ctrl_b), KeyAction::None);
        assert!(leader.is_active());

        // In LeaderPressed state: 'q' -> Quit
        let key_q = KeyEvent::new(KeyCode::Char('q'), KeyModifiers::empty());
        assert_eq!(leader.handle_key(&key_q), KeyAction::Quit);
    }

    #[test]
    fn test_pty_harness_spawn_and_read() {
        let config = HarnessConfig {
            command: "echo".to_string(),
            args: vec!["hello_splash".to_string()],
        };
        let harness = PtyHarness::spawn(&config, 24, 80).unwrap();

        // Wait for output from reader thread
        let mut output = String::new();
        let start = std::time::Instant::now();
        while start.elapsed() < std::time::Duration::from_secs(3) {
            if let Ok(chunk) = harness.output_rx.try_recv() {
                output.push_str(&chunk);
                if output.contains("hello_splash") {
                    break;
                }
            }
            std::thread::sleep(std::time::Duration::from_millis(50));
        }

        assert!(output.contains("hello_splash"));
    }
}
