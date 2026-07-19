use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::Style;
use splash::assert_snapshot;
use splash::pty::HarnessConfig;
use splash::testing::{
    assert_buffer_contains, assert_buffer_matches, assert_buffer_matches_regex,
    format_buffer_grid, TestHarness,
};
use std::time::Instant;

fn create_test_harness(width: u16, height: u16) -> TestHarness {
    let config = HarnessConfig {
        command: "stress_test_app".to_string(),
        args: vec!["--mode".to_string(), "remediation_2".to_string()],
    };
    TestHarness::new(width, height, config)
}

// ============================================================================
// 1. UNUSUAL BUFFER DIMENSIONS (1x1, 200x50, 0-height, 0-width, Clamped >255)
// ============================================================================

#[test]
fn test_testharness_1x1_dimension() {
    let mut harness = create_test_harness(1, 1);
    assert_eq!(harness.app.terminal_size, (1, 1));

    let initial_height = {
        let buffer = harness.render_frame();
        let grid = format_buffer_grid(buffer);
        let lines: Vec<&str> = grid.lines().collect();

        println!("1x1 TestHarness buffer area: {:?}", buffer.area);
        println!("1x1 TestHarness formatted grid lines count: {}", lines.len());
        println!("1x1 TestHarness formatted grid:\n{}", grid);

        assert_eq!(lines.len(), buffer.area.height as usize + 2);
        buffer.area.height
    };

    // Test snapshot assertion on 1x1 harness
    let grid = harness.buffer_snapshot();
    let lines: Vec<&str> = grid.lines().collect();
    assert_snapshot!(&mut harness, &lines);

    // Test key interaction on 1x1 harness
    harness.press_ctrl('b');
    let buffer_leader_height = harness.render_frame().area.height;
    assert_eq!(buffer_leader_height, initial_height);

    harness.press_char('q');
    let buffer_normal_height = harness.render_frame().area.height;
    assert_eq!(buffer_normal_height, initial_height);
}

#[test]
fn test_testharness_200x50_dimension() {
    let mut harness = create_test_harness(200, 50);
    assert_eq!(harness.app.terminal_size, (200, 50));

    harness.inject_pty_output("Rendering in a large 200x50 terminal window...");

    let buffer = harness.render_frame();
    assert_eq!(buffer.area.width, 200);
    assert_eq!(buffer.area.height, 50);

    let grid = format_buffer_grid(buffer);
    let lines: Vec<&str> = grid.lines().collect();

    // 50 rows + top/bottom borders = 52 lines
    assert_eq!(lines.len(), 52);

    let expected_top = format!("┌{}┐", "─".repeat(200));
    let expected_bottom = format!("└{}┘", "─".repeat(200));

    assert_eq!(lines[0], expected_top);
    assert_eq!(lines[51], expected_bottom);

    // Side borders check
    for line in &lines[1..51] {
        assert!(line.starts_with('│'));
        assert!(line.ends_with('│'));
    }

    assert_buffer_contains(buffer, "Rendering in a large 200x50 terminal window...");
    assert_buffer_contains(buffer, "Harness: stress_test_app");
}

#[test]
fn test_testharness_0_height_and_0_width_empty_buffers() {
    // 0x0 Buffer
    let buffer_0x0 = Buffer::empty(Rect::new(0, 0, 0, 0));
    let grid_0x0 = format_buffer_grid(&buffer_0x0);
    assert_eq!(grid_0x0, "┌┐\n└┘");
    let snapshot_0x0 = vec!["┌┐", "└┘"];
    assert_snapshot!(&mut &buffer_0x0, &snapshot_0x0);

    // 80x0 Buffer (nonzero width, zero height)
    let buffer_80x0 = Buffer::empty(Rect::new(0, 0, 80, 0));
    let grid_80x0 = format_buffer_grid(&buffer_80x0);
    let lines_80x0: Vec<&str> = grid_80x0.lines().collect();
    assert_eq!(lines_80x0.len(), 2);
    assert_eq!(lines_80x0[0], format!("┌{}┐", "─".repeat(80)));
    assert_eq!(lines_80x0[1], format!("└{}┘", "─".repeat(80)));
    assert_snapshot!(&mut &buffer_80x0, &lines_80x0);

    // 0x24 Buffer (zero width, nonzero height)
    let buffer_0x24 = Buffer::empty(Rect::new(0, 0, 0, 24));
    let grid_0x24 = format_buffer_grid(&buffer_0x24);
    let lines_0x24: Vec<&str> = grid_0x24.lines().collect();
    assert_eq!(lines_0x24.len(), 26);
    assert_eq!(lines_0x24[0], "┌┐");
    for line in &lines_0x24[1..25] {
        assert_eq!(*line, "││");
    }
    assert_eq!(lines_0x24[25], "└┘");
    assert_snapshot!(&mut &buffer_0x24, &lines_0x24);

    // Test TestHarness with 0x0
    let mut harness_0x0 = create_test_harness(0, 0);
    assert_eq!(harness_0x0.app.terminal_size, (0, 0));
    let frame_0x0 = harness_0x0.render_frame();
    assert_eq!(frame_0x0.area.width, 0);
    assert_eq!(frame_0x0.area.height, 0);
    assert_eq!(harness_0x0.buffer_snapshot(), "┌┐\n└┘");

    // Test TestHarness with 80x0
    let mut harness_80x0 = create_test_harness(80, 0);
    let frame_80x0 = harness_80x0.render_frame();
    assert_eq!(frame_80x0.area.width, 80);
    assert_eq!(frame_80x0.area.height, 0);
    assert_eq!(harness_80x0.buffer_snapshot().lines().count(), 2);

    // Test TestHarness with 0x24
    let mut harness_0x24 = create_test_harness(0, 24);
    let frame_0x24 = harness_0x24.render_frame();
    assert_eq!(frame_0x24.area.width, 0);
    assert_eq!(frame_0x24.area.height, 24);
    assert_eq!(harness_0x24.buffer_snapshot().lines().count(), 26);
}

#[test]
fn test_testharness_dimension_clamping_above_255() {
    // Ratatui TestBackend clamps buffer area dimensions to u8::MAX (255)
    let mut harness = create_test_harness(300, 300);
    assert_eq!(harness.app.terminal_size, (300, 300));

    let buffer = harness.render_frame();
    // TestBackend clamps 300 to 255
    assert_eq!(buffer.area.width, 255);
    assert_eq!(buffer.area.height, 255);

    let grid = harness.buffer_snapshot();
    let lines: Vec<&str> = grid.lines().collect();

    // 255 rows + 2 border lines = 257 lines
    assert_eq!(lines.len(), 257);
    assert_eq!(lines[0], format!("┌{}┐", "─".repeat(255)));
    assert_eq!(lines[256], format!("└{}┘", "─".repeat(255)));
}

// ============================================================================
// 2. ANSI ESCAPES AND CONTROL CHARACTERS IN BUFFER / SNAPSHOT INSPECTION
// ============================================================================

#[test]
fn test_ansi_escapes_and_sgr_formatting_in_pty_output() {
    let mut harness = create_test_harness(80, 10);

    // SGR Color escape codes, Bold, Reset, Cursor movements
    let pty_input = "\x1b[31mRED TEXT\x1b[0m \x1b[1;32mBOLD GREEN\x1b[0m \x1b[44mBLUE BG\x1b[0m\n\
                     Pos (1,1) \x1b[?25h\x1b[?25l\n\
                     Malformed ANSI: \x1b[99999;99999H \x1b[ \x1b[31 \x1b]0;Title\x07";

    harness.inject_pty_output(pty_input);
    let buffer = harness.render_frame();
    let grid = format_buffer_grid(buffer);

    // Verify snapshot grid formatting does not panic or hang
    assert!(!grid.is_empty());
    assert_buffer_contains(buffer, "RED TEXT");
    assert_buffer_contains(buffer, "BOLD GREEN");
    assert_buffer_contains(buffer, "BLUE BG");

    // Verify regex search works over ANSI-containing buffer
    assert_buffer_matches_regex(buffer, r"RED\s+TEXT");
    assert_buffer_matches(buffer, r"Malformed ANSI");
}

#[test]
fn test_non_printable_ascii_control_characters() {
    let mut buffer = Buffer::empty(Rect::new(0, 0, 30, 3));

    // Inject non-printable ASCII control characters into cell symbols
    let control_chars = "\x00\x01\x02\x03\x04\x05\x06\x07\x08\t\n\r\x0b\x0c\x0e\x0f\x10\x1b\x7f";
    buffer.set_string(0, 0, control_chars, Style::default());

    let grid = format_buffer_grid(&buffer);
    let lines: Vec<&str> = grid.lines().collect();

    assert_eq!(lines.len(), 5);
    assert_eq!(lines[0], format!("┌{}┐", "─".repeat(30)));
    assert_eq!(lines[4], format!("└{}┘", "─".repeat(30)));

    // Verify grid formatting completes without panic
    assert_buffer_contains(&buffer, "\x00");
    assert_buffer_contains(&buffer, "\x1b");
    assert_buffer_contains(&buffer, "\x7f");
}

// ============================================================================
// 3. BOUNDARY CONDITION GRIDS (CJK/Emoji right edge, ZWJ, combining chars)
// ============================================================================

#[test]
fn test_cjk_double_width_at_rightmost_column_boundary() {
    // Width 10. Columns 0..9.
    // Place "中" at x=8. It takes cols 8 and 9. Follower cell is at x=9.
    let mut buffer = Buffer::empty(Rect::new(0, 0, 10, 1));
    buffer.set_string(8, 0, "中", Style::default());

    let grid = format_buffer_grid(&buffer);
    let lines: Vec<&str> = grid.lines().collect();

    assert_eq!(lines[0], "┌──────────┐");
    assert_eq!(lines[1], "│        中│");
    assert_eq!(lines[2], "└──────────┘");

    // Width 10. Place "中" at x=9.
    // In Ratatui, a width-2 character set at column width-1 cannot fit.
    // Ratatui set_string will truncate or handle without out-of-bounds error.
    let mut buffer_boundary = Buffer::empty(Rect::new(0, 0, 10, 1));
    buffer_boundary.set_string(9, 0, "中", Style::default());

    let grid_boundary = format_buffer_grid(&buffer_boundary);
    let lines_boundary: Vec<&str> = grid_boundary.lines().collect();

    assert_eq!(lines_boundary.len(), 3);
    assert_eq!(lines_boundary[0], "┌──────────┐");
    assert_eq!(lines_boundary[2], "└──────────┘");
}

#[test]
fn test_emoji_double_width_at_rightmost_column_boundary() {
    // Width 6. Columns 0..5. Place "🦀" at x=4 (cols 4 and 5).
    let mut buffer = Buffer::empty(Rect::new(0, 0, 6, 1));
    buffer.set_string(4, 0, "🦀", Style::default());

    let grid = format_buffer_grid(&buffer);
    let lines: Vec<&str> = grid.lines().collect();

    assert_eq!(lines[0], "┌──────┐");
    assert_eq!(lines[1], "│    🦀│");
    assert_eq!(lines[2], "└──────┘");
}

#[test]
fn test_combining_characters_and_zero_width_joiner() {
    let mut buffer = Buffer::empty(Rect::new(0, 0, 20, 2));

    // ZWJ Sequence (Family emoji 👨‍👩‍👧‍👦)
    let family_emoji = "👨‍👩‍👧‍👦";
    buffer.set_string(0, 0, family_emoji, Style::default());

    // Multiple combining characters U+0300..U+0303 on single base char
    let combining_stack = "a\u{0300}\u{0301}\u{0302}\u{0303}";
    buffer.set_string(0, 1, combining_stack, Style::default());

    let grid = format_buffer_grid(&buffer);
    let lines: Vec<&str> = grid.lines().collect();

    assert_eq!(lines.len(), 4);
    assert_eq!(lines[0], format!("┌{}┐", "─".repeat(20)));
    assert_eq!(lines[3], format!("└{}┘", "─".repeat(20)));

    assert_buffer_contains(&buffer, family_emoji);
    assert_buffer_contains(&buffer, combining_stack);
}

#[test]
fn test_assert_snapshot_macro_with_regex_characters_in_content() {
    let config = HarnessConfig {
        command: "regex_test_cmd".to_string(),
        args: vec![],
    };
    let mut harness = TestHarness::new(50, 3, config);
    harness.inject_pty_output(r"Regex specials: $^.*+?()[]{}|\");

    let top_border = format!("┌{}┐", "─".repeat(50));
    let bottom_border = format!("└{}┘", "─".repeat(50));

    let expected_snapshot = vec![
        top_border.as_str(),
        "│  [1: regex_test_cmd]                             │",
        "│┌ File Tr┐┌ Main Pane (Harness: regex_test_cmd) ─┐│",
        "│└────────┘└──────────────────────────────────────┘│",
        bottom_border.as_str(),
    ];

    assert_snapshot!(&mut harness, &expected_snapshot);
}

// ============================================================================
// 4. STRESS AND STABILITY UNDER LOAD
// ============================================================================

#[test]
fn test_rapid_resize_stress_matrix() {
    let mut harness = create_test_harness(80, 24);

    let test_dimensions = [
        (1, 1),
        (0, 0),
        (1, 0),
        (0, 1),
        (200, 50),
        (255, 255),
        (300, 300),
        (10, 5),
        (80, 24),
    ];

    let start = Instant::now();
    for i in 0..2_000 {
        let (w, h) = test_dimensions[i % test_dimensions.len()];
        harness.resize(w, h);
        let buffer = harness.render_frame();
        let expected_w = w.min(255);
        let expected_h = h.min(255);
        assert_eq!(buffer.area.width, expected_w);
        assert_eq!(buffer.area.height, expected_h);
        if i % 200 == 0 {
            let _grid = harness.buffer_snapshot();
        }
    }
    let elapsed = start.elapsed();
    println!("2,000 rapid size transitions completed in {:?}", elapsed);
}

#[test]
fn test_high_throughput_pty_output_and_snapshot_performance() {
    let mut harness = create_test_harness(100, 30);

    let mut large_payload = String::with_capacity(50_000 * 40);
    for i in 0..50_000 {
        large_payload.push_str(&format!("Line {:05}: ANSI \x1b[32mOK\x1b[0m UTF8 🦀 中文 data batch {}\n", i, i % 100));
    }

    let t0 = Instant::now();
    harness.inject_pty_output(&large_payload);
    let push_time = t0.elapsed();

    let t1 = Instant::now();
    let buffer = harness.render_frame();
    let render_time = t1.elapsed();

    let t2 = Instant::now();
    let grid = format_buffer_grid(buffer);
    let snapshot_time = t2.elapsed();

    println!(
        "50,000 lines payload - Push: {:?}, Render: {:?}, Snapshot: {:?}",
        push_time, render_time, snapshot_time
    );

    assert_eq!(buffer.area.width, 100);
    assert_eq!(buffer.area.height, 30);
    assert!(!grid.is_empty());
}

#[test]
fn test_concurrent_harness_instantiation() {
    let handles: Vec<_> = (0..20)
        .map(|id| {
            std::thread::spawn(move || {
                let mut harness = create_test_harness(80, 24);
                harness.inject_pty_output(&format!("Thread {} active", id));
                harness.press_ctrl('b');
                let grid = harness.buffer_snapshot();
                assert!(grid.contains("[LEADER ACTIVE]"));
                assert!(grid.contains(&format!("Thread {} active", id)));
            })
        })
        .collect();

    for handle in handles {
        handle.join().unwrap();
    }
}
