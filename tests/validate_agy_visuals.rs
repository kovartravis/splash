use std::thread;
use std::time::{Duration, Instant};

use crossterm::event::{KeyCode, KeyModifiers};
use splash::pty::{HarnessConfig, PtyHarness};
use splash::testing::snapshot::format_buffer_grid;
use splash::SplashApp;

#[test]
fn test_validate_agy_visuals() {
    let config = HarnessConfig {
        command: "agy".to_string(),
        args: vec![],
    };

    println!("\n=== VALIDATING AGY VISUAL RENDERING ===");
    println!("Spawning PTY with command: `agy` in 100x24 terminal...");

    let width = 100u16;
    let height = 24u16;

    let harness = match PtyHarness::spawn(&config, height, width) {
        Ok(h) => h,
        Err(err) => {
            panic!("Failed to spawn PTY with `agy`: {}", err);
        }
    };

    let mut app = SplashApp::new(config);
    app.set_size(width, height);

    // Collect PTY output for up to 1.5 seconds
    let start = Instant::now();
    while start.elapsed() < Duration::from_millis(1500) {
        while let Ok(chunk) = harness.output_rx.try_recv() {
            app.push_output_chunk(&chunk);
        }
        thread::sleep(Duration::from_millis(50));
    }

    // Render normal frame
    let backend = ratatui::backend::TestBackend::new(width, height);
    let mut terminal = ratatui::Terminal::new(backend).unwrap();

    terminal
        .draw(|f| {
            app.render(f);
        })
        .unwrap();

    let grid_normal = format_buffer_grid(terminal.backend().buffer());

    println!("\n--- RENDERED TERMINAL SCREEN SNAPSHOT (NORMAL MODE) ---");
    println!("{}", grid_normal);
    println!("--- END NORMAL SNAPSHOT ---\n");

    // Press Ctrl+B to activate leader state
    let key_ctrl_b = crossterm::event::KeyEvent::new(KeyCode::Char('b'), KeyModifiers::CONTROL);
    app.handle_key_event(&key_ctrl_b);

    terminal
        .draw(|f| {
            app.render(f);
        })
        .unwrap();

    let grid_leader = format_buffer_grid(terminal.backend().buffer());

    println!("\n--- RENDERED TERMINAL SCREEN SNAPSHOT (LEADER MODE) ---");
    println!("{}", grid_leader);
    println!("--- END LEADER SNAPSHOT ---\n");

    // Assertions
    assert!(grid_normal.contains("Harness: agy"));
    assert!(grid_leader.contains("[LEADER ACTIVE]"));

    println!("✓ Normal mode title bar: `Harness: agy (Leader: Ctrl+B | Exit: Ctrl+B q)`");
    println!("✓ Leader mode title bar: `Harness: agy (Leader: Ctrl+B | Exit: Ctrl+B q) [LEADER ACTIVE]`");
    println!("✓ Visual validation passed cleanly!");
}
