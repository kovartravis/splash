use std::io::{self, stdout};
use std::time::Duration;
use crossterm::{
    event::{self, Event, KeyCode, KeyEvent, KeyModifiers},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
    backend::CrosstermBackend,
    widgets::{Block, Borders, Paragraph},
    Terminal,
};

fn main() -> io::Result<()> {
    // Set up panic hook to restore terminal if we panic
    let default_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        let _ = restore_terminal();
        default_hook(info);
    }));

    // Initialize terminal
    enable_raw_mode()?;
    let mut stdout = stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // Run TUI loop
    let result = run_app(&mut terminal);

    // Restore terminal
    restore_terminal()?;

    result
}

fn restore_terminal() -> io::Result<()> {
    disable_raw_mode()?;
    execute!(stdout(), LeaveAlternateScreen)?;
    Ok(())
}

fn run_app<B: ratatui::backend::Backend>(terminal: &mut Terminal<B>) -> io::Result<()> {
    loop {
        // Draw the interface
        terminal.draw(|f| {
            let size = f.size();
            let block = Block::default()
                .title("Splash POC - Ticket 1")
                .borders(Borders::ALL);
            let paragraph = Paragraph::new("Splash TUI render loop active. Press 'q' or 'Ctrl+C' to exit.")
                .block(block);
            f.render_widget(paragraph, size);
        })?;

        // Handle input with a poll timeout to avoid burning CPU
        if event::poll(Duration::from_millis(100))? {
            if let Event::Key(key) = event::read()? {
                if should_exit(&key) {
                    break;
                }
            }
        }
    }
    Ok(())
}

fn should_exit(key: &KeyEvent) -> bool {
    // Exit on 'q' or Ctrl+C
    match key.code {
        KeyCode::Char('q') | KeyCode::Char('Q') => true,
        KeyCode::Char('c') | KeyCode::Char('C') => {
            key.modifiers.contains(KeyModifiers::CONTROL)
        }
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
}
