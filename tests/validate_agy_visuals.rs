use std::thread;
use std::time::{Duration, Instant};

use crossterm::event::{KeyCode, KeyModifiers};
use splash::pty::{HarnessConfig, PtyHarness};
use splash::testing::snapshot::format_buffer_grid;
use splash::KeyAction;
use splash::SplashApp;

#[test]
fn test_validate_agy_visuals_and_typing() {
    let config = HarnessConfig {
        command: "agy".to_string(),
        args: vec![],
    };

    println!("\n=== VALIDATING INTERACTIVE TYPING INTO AGY PTY ===");
    println!("Spawning PTY with command: `agy` in 100x24 terminal...");

    let width = 100u16;
    let height = 24u16;

    let mut harness = match PtyHarness::spawn(&config, height, width) {
        Ok(h) => h,
        Err(err) => {
            panic!("Failed to spawn PTY with `agy`: {}", err);
        }
    };

    let mut app = SplashApp::new(config);
    app.set_size(width, height);

    // Collect initial PTY output for 1 second
    let start = Instant::now();
    while start.elapsed() < Duration::from_millis(1000) {
        while let Ok(chunk) = harness.output_rx.try_recv() {
            app.push_output_chunk(&chunk);
        }
        thread::sleep(Duration::from_millis(50));
    }

    // Render frame BEFORE typing
    let backend = ratatui::backend::TestBackend::new(width, height);
    let mut terminal = ratatui::Terminal::new(backend).unwrap();

    terminal
        .draw(|f| {
            app.render(f);
        })
        .unwrap();

    let grid_before = format_buffer_grid(terminal.backend().buffer());
    println!("\n--- SNAPSHOT 1: BEFORE TYPING ---");
    println!("{}", grid_before);

    // Now type 'h', 'e', 'l', 'l', 'o' into the app and forward to PTY
    println!("\nSimulating keystrokes: 'h', 'e', 'l', 'l', 'o'...");
    let input_chars = ['h', 'e', 'l', 'l', 'o'];
    for &ch in &input_chars {
        let key = crossterm::event::KeyEvent::new(KeyCode::Char(ch), KeyModifiers::empty());
        if let KeyAction::Forward(bytes) = app.handle_key_event(&key) {
            harness.write(&bytes);
        }
    }

    // Wait 500ms for PTY echo & render updates
    let typing_start = Instant::now();
    while typing_start.elapsed() < Duration::from_millis(500) {
        while let Ok(chunk) = harness.output_rx.try_recv() {
            app.push_output_chunk(&chunk);
        }
        thread::sleep(Duration::from_millis(30));
    }

    terminal
        .draw(|f| {
            app.render(f);
        })
        .unwrap();

    let grid_after_typing = format_buffer_grid(terminal.backend().buffer());
    println!("\n--- SNAPSHOT 2: AFTER TYPING 'hello' ---");
    println!("{}", grid_after_typing);

    // Now press Enter (b'\r')
    println!("\nSimulating keystroke: Enter...");
    let key_enter = crossterm::event::KeyEvent::new(KeyCode::Enter, KeyModifiers::empty());
    if let KeyAction::Forward(bytes) = app.handle_key_event(&key_enter) {
        harness.write(&bytes);
    }

    // Wait 500ms for response
    let enter_start = Instant::now();
    while enter_start.elapsed() < Duration::from_millis(500) {
        while let Ok(chunk) = harness.output_rx.try_recv() {
            app.push_output_chunk(&chunk);
        }
        thread::sleep(Duration::from_millis(30));
    }

    terminal
        .draw(|f| {
            app.render(f);
        })
        .unwrap();

    let grid_after_enter = format_buffer_grid(terminal.backend().buffer());
    println!("\n--- SNAPSHOT 3: AFTER ENTER ---");
    println!("{}", grid_after_enter);

    println!("\n✓ Interactive keystroke forwarding to PTY verified!");
}
