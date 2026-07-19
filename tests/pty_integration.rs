use splash::assert_snapshot;
use splash::pty::HarnessConfig;
use splash::testing::{assert_buffer_contains, format_buffer_grid, TestHarness};

use splash::tree::FileTree;

fn create_test_config() -> HarnessConfig {
    HarnessConfig {
        command: "pty_cmd".to_string(),
        args: vec!["--verbose".to_string()],
    }
}

fn create_test_harness(width: u16, height: u16) -> TestHarness {
    let temp_dir = std::env::temp_dir().join(format!("splash_pty_empty_tree_{}", std::process::id()));
    let _ = std::fs::create_dir_all(&temp_dir);
    let empty_tree = FileTree::new(&temp_dir).unwrap();
    TestHarness::with_file_tree(width, height, create_test_config(), empty_tree)
}

#[test]
fn test_pty_output_stream_injection_and_raw_accumulation() {
    let mut harness = create_test_harness(60, 6);

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
        "││          ││Hello, Splash PTY!                            ││",
        "││          ││Second line of output                         ││",
        "││          ││Third line of output chunk                    ││",
        "│└──────────┘└──────────────────────────────────────────────┘│",
        "└────────────────────────────────────────────────────────────┘",
    ];
    assert_snapshot!(&mut harness, &expected_lines);
}

#[test]
fn test_terminal_layout_resize_100x30_and_40x10() {
    let mut harness = create_test_harness(80, 24);
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
        "││      ││PTY output line before resize ││",
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
    let mut harness = create_test_harness(60, 6);

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

    // The vt100 terminal with 3 visible rows scrolled all 120 lines through,
    // so the last 3 lines shown on screen are 118, 119, 120.
    assert!(!grid.contains("Line 1\n"));
    assert!(!grid.contains("Line 20\n"));
    // Lines 118..120 are visible in the 3-row main pane inner area
    assert_buffer_contains(buffer, "Line 118");
    assert_buffer_contains(buffer, "Line 119");
    assert_buffer_contains(buffer, "Line 120");
}

#[test]
fn test_harness_content_does_not_overflow_pane_border() {
    // Use a wide terminal to surface width-mismatch bugs.
    // The file tree gets 20% of width; the main pane gets 80%.
    // Content injected into the harness must never bleed past the right border.
    let width = 120u16;
    let height = 10u16;
    let mut harness = create_test_harness(width, height);

    // Inject a line that is exactly as wide as the full terminal — if the PTY
    // is mis-sized (too wide) agy-style separator lines will overflow the border.
    let wide_line: String = "─".repeat(width as usize);
    harness.inject_pty_output(&wide_line);

    let buffer = harness.render_frame();
    let grid = format_buffer_grid(buffer);
    let grid_lines: Vec<&str> = grid.lines().collect();

    // The outer frame is width+2 chars wide (border chars on each side).
    // Every row in the grid must be exactly that width — overflow would make rows longer.
    let expected_row_len = (width + 2) as usize;
    for (i, line) in grid_lines.iter().enumerate() {
        let char_count = line.chars().count();
        assert_eq!(
            char_count, expected_row_len,
            "Row {} has {} chars, expected {} — content overflowed pane border:\n{}",
            i, char_count, expected_row_len, grid
        );
    }

    // The right outer border column must be '│' on every content row
    // (rows 2..height+1), confirming the border was not displaced by overflow.
    for row in &grid_lines[2..=(height as usize)] {
        let last_char = row.chars().last().unwrap_or(' ');
        assert_eq!(
            last_char, '│',
            "Right border displaced on row — content overflow detected:\n{}",
            grid
        );
    }
}

#[test]
fn test_pty_output_with_leader_active_and_resizing() {
    let mut harness = create_test_harness(80, 6);

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
        "││               ││Active PTY session output                                         ││",
        "││               ││                                                                  ││",
        "││               ││                                                                  ││",
        "││               ││                                                                  ││",
        "│└───────────────┘└──────────────────────────────────────────────────────────────────┘│",
        "└─────────────────────────────────────────────────────────────────────────────────────┘",
    ];
    assert_snapshot!(&mut harness, &expected_85x7);
}
