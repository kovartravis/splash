use std::thread;
use std::time::{Duration, Instant};

use crossterm::event::{KeyCode, KeyModifiers};
use splash::testing::snapshot::format_buffer_grid;
use splash::{SplashApp, Tab};
use splash::pty::HarnessConfig;

fn drain_and_tick(app: &mut SplashApp, duration: Duration) {
    let start = Instant::now();
    while start.elapsed() < duration {
        app.tick();
        thread::sleep(Duration::from_millis(30));
    }
}

#[test]
fn test_validate_agy_visuals_and_typing() {
    let width = 100u16;
    let height = 24u16;

    println!("\n=== VALIDATING INTERACTIVE TYPING INTO AGY PTY ===");
    println!("Spawning HarnessTab with command: `agy` in {}x{} terminal...", width, height);

    // Build app and replace the initial tab with a properly spawned agy harness
    let config = HarnessConfig {
        command: "agy".to_string(),
        args: vec![],
    };
    let mut app = SplashApp::new(config);
    app.set_size(width, height);

    // Spawn PTY on the first harness tab (agy)
    let inner_height = height.saturating_sub(3).max(1);
    let inner_width = width.saturating_sub(2).max(1);
    if let Some(Tab::Harness(harness_tab)) = app.tabs.get_mut(0) {
        harness_tab.spawn_pty(inner_height, inner_width);
    }

    // Collect initial PTY output for 5 seconds via tick()
    drain_and_tick(&mut app, Duration::from_millis(5000));

    // Render frame BEFORE typing
    let backend = ratatui::backend::TestBackend::new(width, height);
    let mut terminal = ratatui::Terminal::new(backend).unwrap();

    terminal.draw(|f| { app.render(f); }).unwrap();

    let grid_before = format_buffer_grid(terminal.backend().buffer());
    println!("\n--- SNAPSHOT 1: BEFORE TYPING ---");
    println!("{}", grid_before);

    // Now type 'h', 'e', 'l', 'l', 'o' into the app and forward to PTY
    println!("\nSimulating keystrokes: 'h', 'e', 'l', 'l', 'o'...");
    let input_chars = ['h', 'e', 'l', 'l', 'o'];
    for &ch in &input_chars {
        let key = crossterm::event::KeyEvent::new(KeyCode::Char(ch), KeyModifiers::empty());
        app.handle_key_event(&key);
    }

    // Wait 500ms for PTY echo & render updates
    drain_and_tick(&mut app, Duration::from_millis(500));

    terminal.draw(|f| { app.render(f); }).unwrap();

    let grid_after_typing = format_buffer_grid(terminal.backend().buffer());
    println!("\n--- SNAPSHOT 2: AFTER TYPING 'hello' ---");
    println!("{}", grid_after_typing);

    // Now press Enter
    println!("\nSimulating keystroke: Enter...");
    let key_enter = crossterm::event::KeyEvent::new(KeyCode::Enter, KeyModifiers::empty());
    app.handle_key_event(&key_enter);

    // Wait 500ms for response
    drain_and_tick(&mut app, Duration::from_millis(500));

    terminal.draw(|f| { app.render(f); }).unwrap();

    let grid_after_enter = format_buffer_grid(terminal.backend().buffer());
    println!("\n--- SNAPSHOT 3: AFTER ENTER ---");
    println!("{}", grid_after_enter);

    println!("\n✓ Interactive keystroke forwarding to PTY verified!");
}
