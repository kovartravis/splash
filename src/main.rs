use std::env;
use std::io::{self, stdout};
use std::time::Duration;

use crossterm::{
    event::{self, Event},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{backend::CrosstermBackend, Terminal};
use splash::{parse_args, HarnessConfig, KeyAction, PtyHarness, SplashApp};

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
    let inner_width = size.width.saturating_sub(2).max(1);
    let inner_height = size.height.saturating_sub(2).max(1);

    let mut harness = PtyHarness::spawn(&config, inner_height, inner_width)?;
    let mut app = SplashApp::new(config);
    app.set_size(inner_width, inner_height);

    loop {
        // Drain incoming PTY output
        while let Ok(chunk) = harness.output_rx.try_recv() {
            app.push_output_chunk(&chunk);
        }

        terminal
            .draw(|f| {
                let rect = f.size();
                let inner_w = rect.width.saturating_sub(2).max(1);
                let inner_h = rect.height.saturating_sub(2).max(1);

                harness.resize(inner_h, inner_w);
                app.set_size(inner_w, inner_h);
                app.render(f);
            })
            .map_err(|e| e.to_string())?;

        if event::poll(Duration::from_millis(30)).map_err(|e| e.to_string())? {
            let evt = event::read().map_err(|e| e.to_string())?;
            if let Event::Key(key) = evt {
                match app.handle_key_event(&key) {
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
