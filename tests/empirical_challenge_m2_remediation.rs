use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::Style;
use splash::pty::HarnessConfig;
use splash::testing::{
    assert_buffer_contains, assert_buffer_matches, assert_buffer_matches_regex,
    format_buffer_grid, TestHarness,
};
use splash::assert_snapshot;

// 1. EMPIRICAL CHALLENGE: Non-zero origin Buffer (`Rect::new(x > 0, y > 0, w, h)`)
// `format_buffer_grid` calls `buffer.get(x, y)` assuming origin is (0,0).
// On sub-buffers or offset rects, this panics with "Trying to access position outside the buffer".
#[test]
fn test_nonzero_rect_origin_buffer_panic() {
    let area = Rect::new(5, 10, 10, 3);
    let mut buffer = Buffer::empty(area);
    buffer.set_string(5, 10, "Hello", Style::default());

    let grid = format_buffer_grid(&buffer);
    assert!(grid.contains("Hello"));
}

// 2. EMPIRICAL CHALLENGE: Wide character at rightmost border boundary (x = width - 1)
// When a double-width symbol is directly set at the last column of a buffer,
// `x + sym_w` exceeds `buffer.area.width`, causing the content row to exceed border box width.
#[test]
fn test_manual_cell_symbol_wide_char_at_boundary() {
    let mut buffer = Buffer::empty(Rect::new(0, 0, 4, 1));
    // Manually set cell at x=3 to "中" (display width 2)
    buffer.get_mut(3, 0).set_symbol("中");

    let grid = format_buffer_grid(&buffer);
    let lines: Vec<&str> = grid.lines().collect();

    assert_eq!(lines[0], "┌────┐");
    // Right boundary overflow defense replaces overflowing symbol with space so row length never exceeds width
    assert_eq!(lines[1], "│    │");
    assert_eq!(lines[2], "└────┘");

    // Display width of content row matches top border display width (6)
    let top_border_width = unicode_width::UnicodeWidthStr::width(lines[0]);
    let content_row_width = unicode_width::UnicodeWidthStr::width(lines[1]);
    assert_eq!(top_border_width, 6);
    assert_eq!(content_row_width, 6);
}

/// 3. EMPIRICAL CHALLENGE: Standard CJK & Emoji follower cell skipping
#[test]
fn test_cjk_and_emoji_follower_cell_skipping() {
    let mut buffer = Buffer::empty(Rect::new(0, 0, 8, 2));
    buffer.set_string(0, 0, "中文", Style::default());
    buffer.set_string(0, 1, "🦀🦀", Style::default());

    let grid = format_buffer_grid(&buffer);
    let lines: Vec<&str> = grid.lines().collect();

    assert_eq!(lines[0], "┌────────┐");
    assert_eq!(lines[1], "│中文    │");
    assert_eq!(lines[2], "│🦀🦀    │");
    assert_eq!(lines[3], "└────────┘");

    // All rows must have identical display width (10 columns including borders)
    for line in &lines {
        assert_eq!(unicode_width::UnicodeWidthStr::width(*line), 10);
    }
}

/// 4. EMPIRICAL CHALLENGE: Complex Emojis, ZWJ sequences, Flags, and Modifiers
#[test]
fn test_complex_emoji_zwj_and_skin_tone_alignment() {
    let mut buffer = Buffer::empty(Rect::new(0, 0, 10, 4));

    // Multi-byte & ZWJ emojis
    buffer.set_string(0, 0, "👨‍👩‍👧‍👦", Style::default()); // Family ZWJ
    buffer.set_string(0, 1, "🇺🇸", Style::default());       // US Flag
    buffer.set_string(0, 2, "👋🏽", Style::default());       // Skin tone
    buffer.set_string(0, 3, "👩‍💻", Style::default());       // Woman tech

    let grid = format_buffer_grid(&buffer);
    let lines: Vec<&str> = grid.lines().collect();

    assert_eq!(lines.len(), 6);
    assert_eq!(lines[0], "┌──────────┐");
    assert_eq!(lines[5], "└──────────┘");

    // Verify grid formatting does not crash and produces valid lines
    assert_buffer_contains(&buffer, "👨‍👩‍👧‍👦");
    assert_buffer_contains(&buffer, "🇺🇸");
    assert_buffer_contains(&buffer, "👋🏽");
    assert_buffer_contains(&buffer, "👩‍💻");
}

/// 5. EMPIRICAL CHALLENGE: Zero-Width & Combining Unicode Characters
#[test]
fn test_zero_width_and_combining_characters() {
    let mut buffer = Buffer::empty(Rect::new(0, 0, 6, 2));
    // Zero-width space U+200B, ZWNBSP U+FEFF
    buffer.set_string(0, 0, "A\u{200B}B\u{FEFF}C", Style::default());
    // Combining acute U+0301 attached to 'e'
    buffer.set_string(0, 1, "e\u{0301} test", Style::default());

    let grid = format_buffer_grid(&buffer);
    let lines: Vec<&str> = grid.lines().collect();

    assert_eq!(lines.len(), 4);
    assert_buffer_contains(&buffer, "ABC");
    assert_buffer_contains(&buffer, "e\u{0301} test");
}

// 6. EMPIRICAL CHALLENGE: Control Characters (\n, \r, \t, \0, ESC \x1b)
// Direct cell symbol mutation with newline splits line formatting in grid output.
#[test]
fn test_control_characters_in_cell_symbols() {
    let mut buffer = Buffer::empty(Rect::new(0, 0, 10, 2));
    buffer.get_mut(0, 0).set_symbol("Line\nBreak");
    buffer.get_mut(0, 1).set_symbol("Tab\tEsc\x1b");

    let grid = format_buffer_grid(&buffer);
    let lines: Vec<&str> = grid.lines().collect();

    // Direct insertion of '\n' in cell symbol increases line count from 4 to 5
    assert_eq!(lines.len(), 5);
    assert_eq!(lines[0], "┌──────────┐");
    assert_eq!(lines[1], "│Line");
    assert_eq!(lines[2], "Break │");
    assert_eq!(lines[4], "└──────────┘");
}

/// 7. EMPIRICAL CHALLENGE: Snapshot Macros and Assertion Error Output
#[test]
fn test_snapshot_assertions_coverage() {
    let config = HarnessConfig {
        command: "test".to_string(),
        args: vec![],
    };
    let mut harness = TestHarness::new(20, 5, config);
    harness.inject_pty_output("OK");

    let expected = vec![
        "┌────────────────────┐",
        "│  [1: test]         │",
        "│┌ F┐┌ Main Pane (Ha┐│",
        "││▶ ││OK            ││",
        "││▶ ││              ││",
        "│└──┘└──────────────┘│",
        "└────────────────────┘",
    ];

    // Test assert_snapshot! on TestHarness
    assert_snapshot!(&mut harness, &expected);

    // Test regex assertion aliases
    let buffer = harness.render_frame();
    assert_buffer_matches(buffer, r"OK");
    assert_buffer_matches_regex(buffer, r"Main Pane");
}

/// 8. EMPIRICAL CHALLENGE: Extreme buffer dimensions (0x0, 1x1, 500x1)
#[test]
fn test_extreme_buffer_dimensions() {
    // 0x0 Buffer
    let b0 = Buffer::empty(Rect::new(0, 0, 0, 0));
    assert_eq!(format_buffer_grid(&b0), "┌┐\n└┘");

    // 1x1 Buffer
    let mut b1 = Buffer::empty(Rect::new(0, 0, 1, 1));
    b1.set_string(0, 0, "Z", Style::default());
    let grid1 = format_buffer_grid(&b1);
    assert_eq!(grid1, "┌─┐\n│Z│\n└─┘");

    // 500-col Buffer
    let b500 = Buffer::empty(Rect::new(0, 0, 500, 1));
    let grid500 = format_buffer_grid(&b500);
    let lines500: Vec<&str> = grid500.lines().collect();
    assert_eq!(lines500[0].chars().count(), 502);
}
