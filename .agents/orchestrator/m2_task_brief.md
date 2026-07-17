# Milestone 2 Task Brief for Worker 2

## Objective
Implement Visual Buffer & Snapshot Inspection Utilities in `splash::testing` (`src/testing/snapshot.rs` or `src/testing/mod.rs`), providing plain-text buffer grid formatting with borders and declarative snapshot assertion macros/helpers.

## Requirements
1. **Plain-text Buffer Grid Formatting (`format_buffer_grid`)**:
   - Format `ratatui::buffer::Buffer` (or `TestHarness` snapshot) into plain-text lines enclosed with top, bottom, and side borders (`┌───...───┐`, `│ line... │`, `└───...───┘`) for clear debugging and visual snapshotting.
   - Strip color/ANSI formatting or provide clean text extraction across all buffer rows and columns.
   - Handle Unicode cell widths cleanly.
2. **Snapshot Inspection Helpers & Assertion Macros**:
   - `assert_buffer_contains(buffer: &Buffer, expected: &str)`: Asserts that at least one row or substring in the formatted buffer matches `expected`. Prints formatted buffer grid on failure.
   - `assert_buffer_matches_regex(buffer: &Buffer, pattern: &str)`: Asserts regex pattern match against buffer lines. Prints formatted buffer grid on failure.
   - `assert_snapshot!(harness: &mut TestHarness, expected_lines: &[&str])` or similar macro/function allowing exact line-by-line snapshot verification of rendered frames.
   - Verify snapshot helpers work for titles, borders, `[LEADER ACTIVE]` state indicator, and harness output text.
3. **Integration Tests (`tests/snapshot_inspection.rs`)**:
   - Integration tests exercising `format_buffer_grid`, `assert_buffer_contains`, `assert_buffer_matches_regex`, and snapshot assertions for:
     * Harness title text (`Harness: ...`)
     * Leader key active indicator (`[LEADER ACTIVE]`)
     * PTY output text lines inside rendered buffer
     * Custom terminal sizes (e.g. 80x24, 120x40).
4. **Verification**:
   - Run `cargo test` and ensure 100% of unit and integration tests pass.
   - Run `cargo clippy --all-targets -- -D warnings`.
   - Log action history to neuron memory store (`neuron history add`).
