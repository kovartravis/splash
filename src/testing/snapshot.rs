use ratatui::buffer::Buffer;
use regex::Regex;

use super::TestHarness;

/// Formats a `ratatui::buffer::Buffer` into a plain-text grid with outer borders (`┌...┐`, `│...│`, `└...┘`).
pub fn format_buffer_grid(buffer: &Buffer) -> String {
    let width = buffer.area.width as usize;
    let height = buffer.area.height as usize;

    let mut lines = Vec::with_capacity(height + 2);
    lines.push(format!("┌{}┐", "─".repeat(width)));

    for y in 0..buffer.area.height {
        let mut row_str = String::with_capacity(width);
        let mut x = 0;
        while x < buffer.area.width {
            let cell = buffer.get(buffer.area.x + x, buffer.area.y + y);
            let sym = cell.symbol();
            let sym_w = unicode_width::UnicodeWidthStr::width(sym);
            if sym_w > 1 && x + (sym_w as u16) > buffer.area.width {
                row_str.push(' ');
                x += 1;
            } else {
                row_str.push_str(sym);
                if sym_w > 0 {
                    x += sym_w as u16;
                } else {
                    x += 1;
                }
            }
        }
        lines.push(format!("│{}│", row_str));
    }


    lines.push(format!("└{}┘", "─".repeat(width)));
    lines.join("\n")
}

/// Asserts that the formatted buffer grid contains the `expected` substring.
/// Prints the formatted buffer grid on failure.
pub fn assert_buffer_contains(buffer: &Buffer, expected: &str) {
    let formatted = format_buffer_grid(buffer);
    if !formatted.contains(expected) {
        panic!(
            "Assertion failed: buffer grid does not contain expected substring {:?}.\nFormatted Buffer Grid:\n{}",
            expected, formatted
        );
    }
}

/// Asserts that the formatted buffer grid matches the given regex `pattern`.
/// Prints the formatted buffer grid on failure.
pub fn assert_buffer_matches_regex(buffer: &Buffer, pattern: &str) {
    let re = Regex::new(pattern).unwrap_or_else(|err| {
        panic!("Invalid regex pattern {:?}: {}", pattern, err);
    });
    let formatted = format_buffer_grid(buffer);
    if !re.is_match(&formatted) {
        panic!(
            "Assertion failed: buffer grid does not match regex pattern {:?}.\nFormatted Buffer Grid:\n{}",
            pattern, formatted
        );
    }
}

/// Alias for `assert_buffer_matches_regex`.
pub fn assert_buffer_matches(buffer: &Buffer, pattern: &str) {
    assert_buffer_matches_regex(buffer, pattern);
}

/// Trait allowing types to be converted to a formatted buffer grid for snapshot assertions.
pub trait AsBufferGrid {
    fn to_buffer_grid(&mut self) -> String;
}

impl AsBufferGrid for TestHarness {
    fn to_buffer_grid(&mut self) -> String {
        let buffer = self.render_frame();
        format_buffer_grid(buffer)
    }
}

impl AsBufferGrid for Buffer {
    fn to_buffer_grid(&mut self) -> String {
        format_buffer_grid(self)
    }
}

impl AsBufferGrid for &Buffer {
    fn to_buffer_grid(&mut self) -> String {
        format_buffer_grid(self)
    }
}

/// Asserts that a formatted buffer grid matches the `expected_lines` line-by-line.
pub fn assert_snapshot_grid(actual_grid: &str, expected_lines: &[&str]) {
    let actual_lines: Vec<&str> = actual_grid.lines().collect();

    if actual_lines.len() != expected_lines.len() {
        panic!(
            "Snapshot line count mismatch: expected {} lines, got {} lines.\n\nExpected lines:\n{}\n\nActual grid:\n{}",
            expected_lines.len(),
            actual_lines.len(),
            expected_lines.join("\n"),
            actual_grid
        );
    }

    for (idx, (actual, expected)) in actual_lines.iter().zip(expected_lines.iter()).enumerate() {
        if actual != expected {
            panic!(
                "Snapshot line mismatch at line {}:\n  Expected: {:?}\n  Actual:   {:?}\n\nFormatted Buffer Grid:\n{}",
                idx, expected, actual, actual_grid
            );
        }
    }
}

/// Asserts snapshot matching on a target implementing `AsBufferGrid` (e.g., `TestHarness` or `Buffer`).
pub fn assert_snapshot_target<T: AsBufferGrid>(target: &mut T, expected_lines: &[&str]) {
    let grid = target.to_buffer_grid();
    assert_snapshot_grid(&grid, expected_lines);
}

/// Macro for performing exact line-by-line snapshot verification on a `TestHarness` or `Buffer`.
#[macro_export]
macro_rules! assert_snapshot {
    ($target:expr, $expected_lines:expr) => {
        $crate::testing::assert_snapshot_target($target, $expected_lines);
    };
}

#[cfg(test)]
mod tests {
    use super::*;
    use ratatui::buffer::Buffer;
    use ratatui::layout::Rect;
    use ratatui::widgets::{Block, Borders, Widget};

    #[test]
    fn test_format_buffer_grid_borders_and_content() {
        let mut buffer = Buffer::empty(Rect::new(0, 0, 10, 3));
        let block = Block::default().title("Hi").borders(Borders::ALL);
        block.render(Rect::new(0, 0, 10, 3), &mut buffer);

        let formatted = format_buffer_grid(&buffer);
        let lines: Vec<&str> = formatted.lines().collect();

        assert_eq!(lines.len(), 5);
        assert_eq!(lines[0], "┌──────────┐");
        assert_eq!(lines[1], "│┌Hi──────┐│");
        assert_eq!(lines[2], "││        ││");
        assert_eq!(lines[3], "│└────────┘│");
        assert_eq!(lines[4], "└──────────┘");
    }

    #[test]
    fn test_assert_buffer_contains() {
        let mut buffer = Buffer::empty(Rect::new(0, 0, 12, 3));
        let block = Block::default().title("TestTitle").borders(Borders::ALL);
        block.render(Rect::new(0, 0, 12, 3), &mut buffer);

        assert_buffer_contains(&buffer, "TestTitle");
        assert_buffer_contains(&buffer, "┌────────────┐");
    }

    #[test]
    #[should_panic(expected = "buffer grid does not contain expected substring")]
    fn test_assert_buffer_contains_failure() {
        let buffer = Buffer::empty(Rect::new(0, 0, 5, 2));
        assert_buffer_contains(&buffer, "NONEXISTENT");
    }

    #[test]
    fn test_assert_buffer_matches_regex() {
        let mut buffer = Buffer::empty(Rect::new(0, 0, 15, 3));
        let block = Block::default().title("Splash v1.0").borders(Borders::ALL);
        block.render(Rect::new(0, 0, 15, 3), &mut buffer);

        assert_buffer_matches_regex(&buffer, r"Splash\s+v\d+\.\d+");
        assert_buffer_matches(&buffer, r"┌─+┐");
    }

    #[test]
    #[should_panic(expected = "buffer grid does not match regex pattern")]
    fn test_assert_buffer_matches_regex_failure() {
        let buffer = Buffer::empty(Rect::new(0, 0, 5, 2));
        assert_buffer_matches_regex(&buffer, r"\d{5}");
    }

    #[test]
    fn test_assert_snapshot_grid_success() {
        let buffer = Buffer::empty(Rect::new(0, 0, 6, 2));
        let grid = format_buffer_grid(&buffer);
        let expected = vec!["┌──────┐", "│      │", "│      │", "└──────┘"];
        assert_snapshot_grid(&grid, &expected);
    }

    #[test]
    fn test_format_buffer_grid_offset_area() {
        let mut buffer = Buffer::empty(Rect::new(5, 10, 8, 3));
        let block = Block::default().title("Hi").borders(Borders::ALL);
        block.render(Rect::new(5, 10, 8, 3), &mut buffer);

        let formatted = format_buffer_grid(&buffer);
        let lines: Vec<&str> = formatted.lines().collect();

        assert_eq!(lines.len(), 5);
        assert_eq!(lines[0], "┌────────┐");
        assert_eq!(lines[1], "│┌Hi────┐│");
        assert_eq!(lines[2], "││      ││");
        assert_eq!(lines[3], "│└──────┘│");
        assert_eq!(lines[4], "└────────┘");
    }

    #[test]
    fn test_format_buffer_grid_right_boundary_overflow() {
        let mut buffer = Buffer::empty(Rect::new(0, 0, 5, 1));
        buffer.get_mut(4, 0).set_symbol("🐉");
        let formatted = format_buffer_grid(&buffer);
        let lines: Vec<&str> = formatted.lines().collect();
        assert_eq!(lines[1], "│     │");
    }
}
