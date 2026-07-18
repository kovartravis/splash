use splash::assert_snapshot;
use splash::pty::HarnessConfig;
use splash::testing::{assert_buffer_contains, format_buffer_grid, TestHarness};

fn create_test_config() -> HarnessConfig {
    HarnessConfig {
        command: "pty_cmd".to_string(),
        args: vec!["--verbose".to_string()],
    }
}

#[test]
fn test_pty_output_stream_injection_and_raw_accumulation() {
    let mut harness = TestHarness::new(60, 6, create_test_config());

    // Initially raw_output is empty
    assert!(harness.app.raw_output.is_empty());

    // Inject PTY stream output
    let input_chunk_1 = "Hello, Splash PTY!\r\nSecond line of output";
    harness.inject_pty_output(input_chunk_1);

    // Verify raw output accumulation
    assert_eq!(harness.app.raw_output, input_chunk_1);

    // Verify rendered buffer contains injected output
    let buffer = harness.render_frame();
    assert_buffer_contains(buffer, "Hello, Splash PTY!");
    assert_buffer_contains(buffer, "Second line of output");

    // Inject a second chunk to test stream accumulation
    let input_chunk_2 = "\r\nThird line of output chunk";
    harness.inject_pty_output(input_chunk_2);

    assert_eq!(
        harness.app.raw_output,
        "Hello, Splash PTY!\r\nSecond line of output\r\nThird line of output chunk"
    );

    let buffer = harness.render_frame();
    assert_buffer_contains(buffer, "Third line of output chunk");

    // Verify snapshot grid formatting (60x6 -> 8 lines total)
    let expected_lines = vec![
        "┌────────────────────────────────────────────────────────────┐",
        "│  [1: pty_cmd]                                              │",
        "│┌ File Tree┐┌ Main Pane (Harness: pty_cmd) ────────────────┐│",
        "││(File tree││Hello, Splash PTY!                            ││",
        "││          ││Second line of output                         ││",
        "││          ││Third line of output chunk                    ││",
        "│└──────────┘└──────────────────────────────────────────────┘│",
        "└────────────────────────────────────────────────────────────┘",
    ];
    assert_snapshot!(&mut harness, &expected_lines);
}

#[test]
fn test_terminal_layout_resize_100x30_and_40x10() {
    let mut harness = TestHarness::new(80, 24, create_test_config());
    assert_eq!(harness.app.terminal_size, (80, 24));

    harness.inject_pty_output("PTY output line before resize");

    // Test resize to 100x30
    harness.resize(100, 30);
    assert_eq!(harness.app.terminal_size, (100, 30));

    let buffer_100x30 = harness.render_frame();
    assert_eq!(buffer_100x30.area.width, 100);
    assert_eq!(buffer_100x30.area.height, 30);

    let grid_100x30 = format_buffer_grid(buffer_100x30);
    let grid_lines_100x30: Vec<&str> = grid_100x30.lines().collect();
    // Grid height is 30 + 2 = 32 lines
    assert_eq!(grid_lines_100x30.len(), 32);
    // Outer border top line length is 100 + 2 = 102 chars
    assert_eq!(grid_lines_100x30[0].chars().count(), 102);
    assert!(grid_100x30.contains("PTY output line before resize"));

    // Test resize to 40x10
    harness.resize(40, 10);
    assert_eq!(harness.app.terminal_size, (40, 10));

    let buffer_40x10 = harness.render_frame();
    assert_eq!(buffer_40x10.area.width, 40);
    assert_eq!(buffer_40x10.area.height, 10);

    let grid_40x10 = format_buffer_grid(buffer_40x10);
    let grid_lines_40x10: Vec<&str> = grid_40x10.lines().collect();
    // Grid height is 10 + 2 = 12 lines
    assert_eq!(grid_lines_40x10.len(), 12);
    // Outer border top line length is 40 + 2 = 42 chars
    assert_eq!(grid_lines_40x10[0].chars().count(), 42);
    assert!(grid_40x10.contains("PTY output line before resize"));

    // Assert visual buffer grid correctness via snapshot on 40x10 layout (12 lines total)
    let expected_40x10 = vec![
        "┌────────────────────────────────────────┐",
        "│  [1: pty_cmd]                          │",
        "│┌ File ┐┌ Main Pane (Harness: pty_cmd) ┐│",
        "││(File ││PTY output line before resize ││",
        "││      ││                              ││",
        "││      ││                              ││",
        "││      ││                              ││",
        "││      ││                              ││",
        "││      ││                              ││",
        "││      ││                              ││",
        "│└──────┘└──────────────────────────────┘│",
        "└────────────────────────────────────────┘",
    ];
    assert_snapshot!(&mut harness, &expected_40x10);
}

#[test]
fn test_pty_output_truncation_large_stream() {
    let mut harness = TestHarness::new(60, 6, create_test_config());

    // Inject 120 lines of output
    let mut lines = Vec::new();
    for i in 1..=120 {
        lines.push(format!("Line {}", i));
    }
    let full_output = lines.join("\r\n");
    harness.inject_pty_output(&full_output);

    // Raw output has all 120 lines
    assert_eq!(harness.app.raw_output.lines().count(), 120);

    // SplashApp::render truncates output to fit 3 visible rows
    let buffer = harness.render_frame();
    let grid = format_buffer_grid(buffer);

    // Line 1 is scrolled off top and absent from screen
    assert!(!grid.contains("Line 1\n"));
    assert!(!grid.contains("Line 20\n"));
    // Lines 115..117 are visible in 3-row main pane inner area
    assert_buffer_contains(buffer, "Line 115");
    assert_buffer_contains(buffer, "Line 116");
    assert_buffer_contains(buffer, "Line 117");
}

#[test]
fn test_pty_output_with_leader_active_and_resizing() {
    let mut harness = TestHarness::new(80, 6, create_test_config());

    harness.inject_pty_output("Active PTY session output");

    // Activate leader key
    harness.press_ctrl('b');
    assert!(harness.app.leader_state.is_active());

    let buffer = harness.render_frame();
    assert_buffer_contains(buffer, "[LEADER ACTIVE]");
    assert_buffer_contains(buffer, "Active PTY session output");

    // Resize while leader is active to 85x7
    harness.resize(85, 7);
    assert_eq!(harness.app.terminal_size, (85, 7));

    let expected_85x7 = vec![
        "┌─────────────────────────────────────────────────────────────────────────────────────┐",
        "│  [1: pty_cmd]                                                                       │",
        "│┌ File Tree ────┐┌ Main Pane (Harness: pty_cmd) [LEADER ACTIVE] ────────────────────┐│",
        "││(File tree plac││Active PTY session output                                         ││",
        "││               ││                                                                  ││",
        "││               ││                                                                  ││",
        "││               ││                                                                  ││",
        "│└───────────────┘└──────────────────────────────────────────────────────────────────┘│",
        "└─────────────────────────────────────────────────────────────────────────────────────┘",
    ];
    assert_snapshot!(&mut harness, &expected_85x7);
}
