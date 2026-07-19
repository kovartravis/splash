# Original User Request

## 2026-07-17T01:24:17Z

Build a headless visual test harness and snapshot validation suite for Splash using `ratatui::backend::TestBackend` to automatically verify TUI frame layouts, PTY output rendering, leader key states, and layout responsiveness.

Working directory: /Users/Travis/Repos/splash
Integrity mode: development

## Requirements

### R1. Headless Test Harness
Implement a headless rendering harness abstraction built on `ratatui::backend::TestBackend` that allows rendering Splash UI components and main loop views offscreen without raw terminal access.

### R2. Visual Buffer & Snapshot Inspection
Provide helper assertions and formatters to capture rendered terminal buffer grids into readable text snapshots, allowing exact string or regex assertions against rendered lines, borders, titles, and PTY output text.

### R3. Interactive State & Leader Key Integration Tests
Write end-to-end integration tests in `tests/` that simulate key events (`Ctrl+B`, `q`, character keys), PTY stream chunks, and layout resize events, asserting correct terminal frame updates.

## Acceptance Criteria

### Headless Verification
- [ ] Integration test suite in `tests/` compiles and runs via `cargo test`.
- [ ] Test harness successfully initializes a `TestBackend` with custom dimensions (e.g. 80x24, 120x40).
- [ ] `cargo test` passes reproducibly in non-interactive CI environments without opening a terminal window.

### Visual Buffer Inspection
- [ ] Helper utilities format `TestBackend` buffer contents into plain-text lines with borders for easy debugging.
- [ ] Snapshot assertions verify title text, leader key active indicator (`[LEADER ACTIVE]`), and harness output lines.
