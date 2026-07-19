# Project: Splash Headless Visual Test Harness & Snapshot Validation Suite

## Architecture
- **App Core (`src/lib.rs`, `src/app.rs`, `src/leader.rs`, `src/pty.rs`)**:
  Refactor Splash UI state and rendering into a library target (`src/lib.rs`) with `SplashApp` struct supporting generic Ratatui `Backend` rendering (`render<B: Backend>(&mut self, frame: &mut Frame<B>)`), `LeaderState` machine, and `PtyHarness`.
- **Headless Test Harness (`src/testing/` or `splash::testing`)**:
  `TestHarness` wrapping `Terminal<TestBackend>`, `SplashApp`, and PTY stream mocking/injection, supporting custom terminal dimensions (80x24, 120x40, etc.).
- **Visual Buffer & Snapshot Inspection (`splash::testing::snapshot`)**:
  Grid formatter (`format_buffer_grid(&Buffer) -> String`) with plain-text borders (`┌...┐`, `│...│`, `└...┘`), and snapshot assertion macros (`assert_buffer_contains`, `assert_buffer_matches`, `assert_snapshot`).
- **Integration Test Suite (`tests/`)**:
  - `tests/headless_harness.rs`: Custom dimensions and offscreen rendering tests.
  - `tests/snapshot_inspection.rs`: Title text, borders, and `[LEADER ACTIVE]` snapshot assertions.
  - `tests/interactive_leader_keys.rs`: Key event simulation (`Ctrl+B`, `q`, char keys) and leader state assertions.
  - `tests/pty_integration.rs`: PTY stream chunk injection, output rendering, and layout resize events.

## Milestones
| # | Name | Scope | Dependencies | Status |
|---|------|-------|-------------|--------|
| 0 | Exploration & Architecture Analysis | Explore codebase, event loop, and design test harness | none | DONE |
| 1 | Library Decoupling & Headless Test Harness | Expose `src/lib.rs`, `SplashApp`, and `TestHarness` with `TestBackend` | M0 | DONE |
| 2 | Visual Buffer & Snapshot Inspection | Grid formatter with borders & snapshot assertion utilities | M1 | DONE |
| 3 | Interactive & Integration Test Suite | Integration tests in `tests/` for key events, leader states, PTY, resize | M2 | PLANNED |
| 4 | Final Verification & Forensic Audit | Full `cargo test` run, Challenger verification, Forensic Audit | M3 | PLANNED |


## Interface Contracts
### `SplashApp`
```rust
pub struct SplashApp {
    pub config: HarnessConfig,
    pub leader_state: LeaderState,
    pub raw_output: String,
    pub terminal_size: (u16, u16),
    // ...
}
impl SplashApp {
    pub fn new(config: HarnessConfig) -> Self;
    pub fn render<B: Backend>(&mut self, frame: &mut Frame<B>);
    pub fn handle_key_event(&mut self, key: &KeyEvent) -> KeyAction;
    pub fn push_output_chunk(&mut self, text: &str);
    pub fn set_size(&mut self, width: u16, height: u16);
}
```

### `TestHarness`
```rust
pub struct TestHarness {
    pub terminal: Terminal<TestBackend>,
    pub app: SplashApp,
}
impl TestHarness {
    pub fn new(width: u16, height: u16, config: HarnessConfig) -> Self;
    pub fn send_key(&mut self, code: KeyCode, modifiers: KeyModifiers) -> KeyAction;
    pub fn press_char(&mut self, c: char) -> KeyAction;
    pub fn press_ctrl(&mut self, c: char) -> KeyAction;
    pub fn inject_pty_output(&mut self, text: &str);
    pub fn resize(&mut self, width: u16, height: u16);
    pub fn render_frame(&mut self) -> &Buffer;
    pub fn buffer_snapshot(&mut self) -> String;
}
```

### Visual Buffer Inspection API
```rust
pub fn format_buffer_grid(buffer: &Buffer) -> String;
pub fn assert_buffer_contains(buffer: &Buffer, expected: &str);
pub fn assert_buffer_matches_regex(buffer: &Buffer, pattern: &str);
```

## Code Layout
- `src/lib.rs` (Root module re-exporting `app`, `leader`, `pty`, `testing`)
- `src/main.rs` (Binary entry point invoking `SplashApp` / `run_splash`)
- `src/app.rs` (`SplashApp` state & generic rendering)
- `src/leader.rs` (`LeaderState`, `KeyAction`, `key_event_to_bytes`)
- `src/pty.rs` (`PtyHarness`, `HarnessConfig`, `parse_args`)
- `src/testing/mod.rs` (`TestHarness`, grid formatter, snapshot helpers)
- `tests/headless_harness.rs`
- `tests/snapshot_inspection.rs`
- `tests/interactive_leader_keys.rs`
- `tests/pty_integration.rs`
