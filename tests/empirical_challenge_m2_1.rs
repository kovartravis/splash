use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::Style;
use splash::testing::{
    assert_buffer_contains, assert_buffer_matches_regex,
    format_buffer_grid,
};
use splash::assert_snapshot;

#[test]
fn test_edge_case_empty_0x0_buffer() {
    let buffer = Buffer::empty(Rect::new(0, 0, 0, 0));
    let grid = format_buffer_grid(&buffer);
    assert_eq!(grid, "┌┐\n└┘");

    // Snapshot assertion on 0x0 buffer
    let expected = vec!["┌┐", "└┘"];
    assert_snapshot!(&mut &buffer, &expected);
}

#[test]
fn test_edge_case_zero_width_nonzero_height_buffer() {
    let buffer = Buffer::empty(Rect::new(0, 0, 0, 3));
    let grid = format_buffer_grid(&buffer);
    let lines: Vec<&str> = grid.lines().collect();
    assert_eq!(lines.len(), 5); // top + 3 rows + bottom
    assert_eq!(lines[0], "┌┐");
    assert_eq!(lines[1], "││");
    assert_eq!(lines[2], "││");
    assert_eq!(lines[3], "││");
    assert_eq!(lines[4], "└┘");
}

#[test]
fn test_edge_case_nonzero_width_zero_height_buffer() {
    let buffer = Buffer::empty(Rect::new(0, 0, 5, 0));
    let grid = format_buffer_grid(&buffer);
    let lines: Vec<&str> = grid.lines().collect();
    assert_eq!(lines.len(), 2); // top + bottom
    assert_eq!(lines[0], "┌─────┐");
    assert_eq!(lines[1], "└─────┘");
}

#[test]
fn test_edge_case_single_cell_1x1_buffer() {
    let mut buffer = Buffer::empty(Rect::new(0, 0, 1, 1));
    buffer.set_string(0, 0, "X", Style::default());
    let expected = vec!["┌─┐", "│X│", "└─┘"];
    assert_snapshot!(&mut &buffer, &expected);
}

#[test]
fn test_cjk_double_width_follower_cell_behavior() {
    let mut buffer = Buffer::empty(Rect::new(0, 0, 6, 1));
    buffer.set_string(0, 0, "中文", Style::default());
    let grid = format_buffer_grid(&buffer);
    let lines: Vec<&str> = grid.lines().collect();

    // Observe: Ratatui stores "中" at x=0 and " " at follower cell x=1.
    // format_buffer_grid skips follower cells for wide characters.
    assert_eq!(lines[0], "┌──────┐");
    assert_eq!(lines[1], "│中文  │");
    assert_eq!(lines[2], "└──────┘");
}

#[test]
fn test_emoji_double_width_follower_cell_behavior() {
    let mut buffer = Buffer::empty(Rect::new(0, 0, 6, 1));
    buffer.set_string(0, 0, "🦀", Style::default());
    let grid = format_buffer_grid(&buffer);
    let lines: Vec<&str> = grid.lines().collect();

    // Emoji "🦀" (display width 2) follower cell space at x=1 is skipped.
    assert_eq!(lines[0], "┌──────┐");
    assert_eq!(lines[1], "│🦀    │");
    assert_eq!(lines[2], "└──────┘");
}

#[test]
fn test_edge_case_cjk_overflow_at_boundary() {
    // Buffer width 3: "中" at x=0 takes 2 cols (x=0, x=1).
    // Placing "文" at x=2 takes 2 cols, which overflows width 3.
    let mut buffer = Buffer::empty(Rect::new(0, 0, 3, 1));
    buffer.set_string(0, 0, "中文", Style::default());
    let grid = format_buffer_grid(&buffer);
    let lines: Vec<&str> = grid.lines().collect();

    assert_eq!(lines[0], "┌───┐");
    // "中" rendered at x=0, follower space at x=1 is skipped, x=2 space.
    assert_eq!(lines[1], "│中 │");
    assert_eq!(lines[2], "└───┘");
}

#[test]
fn test_edge_case_combining_unicode_characters() {
    let mut buffer = Buffer::empty(Rect::new(0, 0, 5, 1));
    // 'e' + combining acute accent U+0301 = "é"
    let combining_e = "e\u{0301}";
    buffer.set_string(0, 0, combining_e, Style::default());

    let _grid = format_buffer_grid(&buffer);
    assert_buffer_contains(&buffer, combining_e);
}

#[test]
fn test_edge_case_extremely_wide_buffer_1000_cols() {
    let width = 1000;
    let mut buffer = Buffer::empty(Rect::new(0, 0, width, 1));
    buffer.set_string(0, 0, "START", Style::default());
    buffer.set_string(995, 0, "END!!", Style::default());

    let grid = format_buffer_grid(&buffer);
    let lines: Vec<&str> = grid.lines().collect();

    assert_eq!(lines.len(), 3);
    let expected_top = format!("┌{}┐", "─".repeat(1000));
    let expected_bottom = format!("└{}┘", "─".repeat(1000));
    assert_eq!(lines[0], expected_top);
    assert_eq!(lines[2], expected_bottom);

    assert_buffer_contains(&buffer, "START");
    assert_buffer_contains(&buffer, "END!!");
}

#[test]
fn test_edge_case_u16_max_wide_buffer() {
    // Test large width u16 (e.g. 5000)
    let width = 5000;
    let buffer = Buffer::empty(Rect::new(0, 0, width, 1));
    let grid = format_buffer_grid(&buffer);
    let lines: Vec<&str> = grid.lines().collect();

    assert_eq!(lines[0].chars().count(), 5002); // 5000 dashes + '┌' + '┐'
}

#[test]
fn test_snapshot_assertions_multiline_and_regex() {
    let mut buffer = Buffer::empty(Rect::new(0, 0, 10, 2));
    buffer.set_string(0, 0, "Line 1", Style::default());
    buffer.set_string(0, 1, "Line 2", Style::default());

    // Multiline contains check
    let sub = "│Line 1    │\n│Line 2    │";
    assert_buffer_contains(&buffer, sub);

    // Multiline regex check using (?s) dot matches all
    assert_buffer_matches_regex(&buffer, r"(?s)Line 1.*Line 2");
}

#[test]
fn test_snapshot_assertions_error_messages() {
    let buffer = Buffer::empty(Rect::new(0, 0, 4, 1));

    // Test assert_snapshot line count mismatch message
    let wrong_line_count = vec!["┌────┐"];
    let err = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        assert_snapshot!(&mut &buffer, &wrong_line_count);
    }));
    assert!(err.is_err());

    // Test assert_snapshot content mismatch message
    let wrong_content = vec!["┌────┐", "│FAIL│", "└────┘"];
    let err2 = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        assert_snapshot!(&mut &buffer, &wrong_content);
    }));
    assert!(err2.is_err());
}
