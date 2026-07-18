use splash::pty::HarnessConfig;
use splash::testing::{
    assert_buffer_contains, assert_buffer_matches, assert_buffer_matches_regex,
    format_buffer_grid, TestHarness,
};
use splash::assert_snapshot;

#[test]
fn test_snapshot_inspection_80x24_title_and_pty_output() {
    let config = HarnessConfig {
        command: "bash".to_string(),
        args: vec![],
    };
    let mut harness = TestHarness::new(80, 24, config);
    harness.inject_pty_output("Welcome to Splash Visual Harness!");

    let buffer = harness.render_frame();
    let grid = format_buffer_grid(buffer);
    let lines: Vec<&str> = grid.lines().collect();

    // Total lines = 24 rows + top border + bottom border = 26
    assert_eq!(lines.len(), 26);

    // Verify top border width (80 chars between corner characters)
    let expected_top_border = format!("┌{}┐", "─".repeat(80));
    assert_eq!(lines[0], expected_top_border);

    // Verify bottom border width
    let expected_bottom_border = format!("└{}┘", "─".repeat(80));
    assert_eq!(lines[25], expected_bottom_border);

    // Verify side borders on every row line
    for line in &lines[1..25] {
        assert!(line.starts_with('│'));
        assert!(line.ends_with('│'));
    }

    // Verify title text and PTY output via assertion helpers
    assert_buffer_contains(buffer, "Main Pane (Harness: bash)");
    assert_buffer_contains(buffer, "Welcome to Splash Visual Harness!");

    // Verify regex matching helpers
    assert_buffer_matches_regex(buffer, r"Main Pane \(Harness:\s+bash\)");
    assert_buffer_matches_regex(buffer, r"Welcome\s+to\s+Splash");
    assert_buffer_matches(buffer, r"┌─+┐");
}

#[test]
fn test_snapshot_inspection_120x40_custom_dimensions() {
    let config = HarnessConfig {
        command: "python3".to_string(),
        args: vec!["-i".to_string()],
    };
    let mut harness = TestHarness::new(120, 40, config);
    harness.inject_pty_output("Python 3.12.0 interactive session\n>>> ");

    let buffer = harness.render_frame();
    let grid = format_buffer_grid(buffer);
    let lines: Vec<&str> = grid.lines().collect();

    // Total lines = 40 rows + top border + bottom border = 42
    assert_eq!(lines.len(), 42);

    // Verify top & bottom border lengths (120 chars inside corners)
    let expected_top = format!("┌{}┐", "─".repeat(120));
    let expected_bottom = format!("└{}┘", "─".repeat(120));
    assert_eq!(lines[0], expected_top);
    assert_eq!(lines[41], expected_bottom);

    // Assertions on content
    assert_buffer_contains(buffer, "Main Pane (Harness: python3)");
    assert_buffer_contains(buffer, "Python 3.12.0 interactive session");
    assert_buffer_contains(buffer, ">>> ");

    assert_buffer_matches_regex(buffer, r"Python\s+\d+\.\d+\.\d+");
    assert_buffer_matches_regex(buffer, r">>>");
}

#[test]
fn test_snapshot_inspection_leader_active_indicator() {
    let config = HarnessConfig {
        command: "zsh".to_string(),
        args: vec![],
    };
    let mut harness = TestHarness::new(80, 24, config);

    // Initial state: LEADER is not active
    let buffer1 = harness.render_frame();
    let grid1 = format_buffer_grid(buffer1);
    assert!(!grid1.contains("[LEADER ACTIVE]"));
    assert_buffer_contains(buffer1, "Main Pane (Harness: zsh)");

    // Press Ctrl+B to activate leader state
    harness.press_ctrl('b');
    let buffer2 = harness.render_frame();
    let grid2 = format_buffer_grid(buffer2);

    assert!(grid2.contains("[LEADER ACTIVE]"));
    assert_buffer_contains(buffer2, "[LEADER ACTIVE]");
    assert_buffer_matches_regex(buffer2, r"\[LEADER ACTIVE\]");

    // Exit leader state by pressing 'q'
    harness.press_char('q');
    let buffer3 = harness.render_frame();
    let grid3 = format_buffer_grid(buffer3);
    assert!(!grid3.contains("[LEADER ACTIVE]"));
}

use splash::tree::FileTree;

fn create_test_harness(width: u16, height: u16, config: HarnessConfig) -> TestHarness {
    let temp_dir = std::env::temp_dir().join(format!("splash_snap_empty_tree_{}", std::process::id()));
    let _ = std::fs::create_dir_all(&temp_dir);
    let empty_tree = FileTree::new(&temp_dir).unwrap();
    TestHarness::with_file_tree(width, height, config, empty_tree)
}

#[test]
fn test_assert_snapshot_macro_exact_matching() {
    let config = HarnessConfig {
        command: "sh".to_string(),
        args: vec![],
    };
    let mut harness = create_test_harness(75, 4, config);
    harness.inject_pty_output("Line 1\r\nLine 2");

    let expected_top_border = format!("┌{}┐", "─".repeat(75));
    let expected_bottom_border = format!("└{}┘", "─".repeat(75));

    let snapshot_lines = vec![
        expected_top_border.as_str(),
        "│  [1: sh]                                                                  │",
        "│┌ File Tree ──┐┌ Main Pane (Harness: sh) ─────────────────────────────────┐│",
        "││             ││Line 2                                                    ││",
        "│└─────────────┘└──────────────────────────────────────────────────────────┘│",
        expected_bottom_border.as_str(),
    ];

    assert_snapshot!(&mut harness, &snapshot_lines);

    // Trigger Leader state and assert updated snapshot
    harness.press_ctrl('b');
    let leader_snapshot_lines = vec![
        expected_top_border.as_str(),
        "│  [1: sh]                                                                  │",
        "│┌ File Tree ──┐┌ Main Pane (Harness: sh) [LEADER ACTIVE] ─────────────────┐│",
        "││             ││Line 2                                                    ││",
        "│└─────────────┘└──────────────────────────────────────────────────────────┘│",
        expected_bottom_border.as_str(),
    ];

    assert_snapshot!(&mut harness, &leader_snapshot_lines);
}

#[test]
fn test_assert_buffer_contains_failure_panics() {
    let config = HarnessConfig {
        command: "echo".to_string(),
        args: vec![],
    };
    let mut harness = TestHarness::new(40, 5, config);
    let buffer = harness.render_frame();

    let result = std::panic::catch_unwind(|| {
        assert_buffer_contains(buffer, "EXPECTED_STRING_THAT_DOES_NOT_EXIST");
    });
    assert!(result.is_err());
}

#[test]
fn test_assert_buffer_matches_regex_failure_panics() {
    let config = HarnessConfig {
        command: "echo".to_string(),
        args: vec![],
    };
    let mut harness = TestHarness::new(40, 5, config);
    let buffer = harness.render_frame();

    let result = std::panic::catch_unwind(|| {
        assert_buffer_matches_regex(buffer, r"MISSING_PATTERN_\d{10}");
    });
    assert!(result.is_err());
}

#[test]
fn test_assert_snapshot_failure_panics_on_mismatch() {
    let config = HarnessConfig {
        command: "echo".to_string(),
        args: vec![],
    };
    let mut harness = TestHarness::new(20, 3, config);

    let wrong_lines = vec![
        "┌────────────────────┐",
        "│ WRONG CONTENT      │",
        "│ WRONG CONTENT      │",
        "│ WRONG CONTENT      │",
        "└────────────────────┘",
    ];

    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        assert_snapshot!(&mut harness, &wrong_lines);
    }));
    assert!(result.is_err());
}
