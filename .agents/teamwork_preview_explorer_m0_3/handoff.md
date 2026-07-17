# Explorer 3 Handoff Report — Test Harness & Snapshot Design

## 1. Observation

- **Dependencies (`Cargo.toml:6-10`)**:
  - `ratatui = "0.26"` (`Cargo.toml:7`)
  - `crossterm = { version = "0.27", features = ["event-stream"] }` (`Cargo.toml:8`)
  - `portable-pty = "0.8"` (`Cargo.toml:9`)
- **Main TUI Loop (`src/main.rs:223-285`)**:
  - Hardcodes `CrosstermBackend::new(stdout)` (`src/main.rs:227`).
  - Terminal draw closure (`src/main.rs:254-266`) directly renders `Block` and `Paragraph` onto the crossterm terminal frame.
- **Key Event & Leader Handling (`src/main.rs:35-109`)**:
  - `LeaderState` handles `Ctrl+B` chords and produces `KeyAction` (`src/main.rs:54-80`).
  - `key_event_to_bytes` translates Crossterm `KeyEvent` to byte vectors for PTY input (`src/main.rs:83-109`).
- **PTY Harness (`src/main.rs:111-191`)**:
  - `PtyHarness::spawn` creates PTY pair using `portable-pty` native PTY system, spawns child process, and starts background output reader thread (`src/main.rs:120-169`).
- **Neuron Memory Query**:
  - Command `neuron learn query "test harness snapshot ratatui"` executed with result: `{"results":[],"project":"splash","query":"test harness snapshot ratatui"}`.

## 2. Logic Chain

1. **Ratatui `TestBackend` Availability**:
   - `ratatui = "0.26"` provides `ratatui::backend::TestBackend` and `ratatui::buffer::Buffer`.
   - `TestBackend::new(width, height)` provides an in-memory buffer that can be drawn to via standard `Terminal::new(backend)` without stdout or raw mode initialization.
2. **Decoupling Application State from IO**:
   - Currently `run_splash` in `src/main.rs:223-285` intertwines `CrosstermBackend`, PTY output reading, and terminal rendering inside a single function.
   - Refactoring state management into `SplashApp` with a generic `render<B: Backend>(&mut self, frame: &mut Frame<B>, size: (u16, u16))` method allows `TestBackend` to render exact UI frames identical to `CrosstermBackend`.
3. **Headless Test Harness Abstraction (`TestHarness`)**:
   - `TestHarness` wraps `Terminal<TestBackend>`, `SplashApp`, and `MockPty`.
   - Providing helper methods `send_key`, `press_char`, `press_ctrl`, `inject_pty_output`, `resize`, and `snapshot_string` enables concise, declarative integration tests for Splash UI without opening real terminal windows or requiring interactive input.
4. **Visual Buffer Grid Formatting & Snapshot Assertions**:
   - Buffer cells in `ratatui::buffer::Buffer` map (x,y) coordinates to cell symbols.
   - Formatting with outer borders (`┌...┐`, `│...│`, `└...┘`) produces unambiguous, fixed-width text snapshots.
   - Snapshot assertions (`assert_snapshot!`, `assert_buffer_contains`, `assert_buffer_matches`) paired with `pretty_assertions` provide readable diffs in test output.
5. **Integration Test Layout**:
   - Placing shared test utilities under `tests/common/` (with `harness.rs` and `snapshots.rs`) and test cases in `tests/leader_keys.rs`, `tests/pty_integration.rs`, and `tests/visual_layout.rs` complies with standard Cargo integration test design.

## 3. Caveats

- **ANSI Color Snapshotting**: Plain-text buffer snapshots examine cell symbols (`cell.symbol()`) and strip color codes for diff readability. If testing color-coded tab borders or active leader highlights specifically, style inspection helpers (`cell.fg`, `cell.bg`, `cell.modifier`) should be added as secondary assertions.
- **Unicode Column Width**: Cells containing multi-width characters (e.g. CJK or emojis) take 2 columns in terminal rendering; `ratatui` handles this by placing empty string symbols in trailing cells. Formatting code must handle cell iteration directly without assuming 1 byte per symbol.

## 4. Conclusion

Splash's dependencies (`ratatui = "0.26"`) fully support in-memory TUI testing via `TestBackend` and `Buffer`. By extracting rendering logic into a generic `SplashApp` struct, Splash can adopt a headless `TestHarness` with visual snapshot assertions and mock/real PTY helpers. All concrete designs, APIs, and module structures have been documented in `/Users/Travis/Repos/splash/.agents/teamwork_preview_explorer_m0_3/analysis.md`.

## 5. Verification Method

1. **Inspect Analysis Document**:
   Read `/Users/Travis/Repos/splash/.agents/teamwork_preview_explorer_m0_3/analysis.md` to review API signatures and struct definitions for `SplashApp`, `TestHarness`, `format_buffer_grid`, `assert_snapshot!`, `MockPty`, and `RealPtyHarnessTest`.
2. **Cargo Verification (Post-Implementation)**:
   - Run `cargo test` to execute unit and integration tests.
   - Run `cargo test --test visual_layout` to verify visual buffer snapshot assertions.
