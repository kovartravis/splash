use std::env;
use std::io::{self, stdout};
use std::time::Duration;

use crossterm::{
    event::{self, Event},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{backend::CrosstermBackend, Terminal};
use splash::{parse_args, HarnessConfig, KeyAction, SplashApp};

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

    let mut app = SplashApp::new(config.clone());
    app.set_size(size.width, size.height);

    // Spawn a PTY for the initial harness tab using the same geometry as set_size
    if let Some(splash::Tab::Harness(harness_tab)) = app.tabs.get_mut(0) {
        let inner_height = size.height.saturating_sub(3).max(1);
        let inner_width = (size.width * 80 / 100).saturating_sub(2).max(1);
        harness_tab.spawn_pty(inner_height, inner_width);
    }

    loop {
        app.tick();

        terminal
            .draw(|f| {
                app.render(f);
            })
            .map_err(|e| e.to_string())?;

        if event::poll(Duration::from_millis(30)).map_err(|e| e.to_string())? {
            let evt = event::read().map_err(|e| e.to_string())?;
            match evt {
                Event::Key(key) => {
                    if let KeyAction::Quit = app.handle_key_event(&key) {
                        break;
                    }
                }
                Event::Resize(new_w, new_h) => {
                    app.set_size(new_w, new_h);
                }
                _ => {}
            }
        }
    }

    restore_terminal().map_err(|e| e.to_string())?;
    Ok(())
}
