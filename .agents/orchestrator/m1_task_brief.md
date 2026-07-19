# Milestone 1 Task Brief for Worker 1

## Objective
Decouple Splash into a library target (`src/lib.rs`) and implement the `SplashApp` rendering abstraction and `TestHarness` built on `ratatui::backend::TestBackend`.

## Requirements & Design
1. **Refactor Crate Structure**:
   - `src/lib.rs`: Expose `app`, `leader`, `pty`, `testing`.
   - `src/leader.rs`: Move `LeaderState`, `KeyAction`, `key_event_to_bytes` from `src/main.rs`.
   - `src/pty.rs`: Move `HarnessConfig`, `parse_args`, `PtyHarness` from `src/main.rs`.
   - `src/app.rs`: Define `SplashApp` wrapping `HarnessConfig`, `LeaderState`, output buffer, and terminal size. Provide `pub fn render<B: Backend>(&mut self, frame: &mut Frame<B>)`, `pub fn handle_key_event(&mut self, key: &KeyEvent) -> KeyAction`, `pub fn push_output_chunk(&mut self, text: &str)`, `pub fn set_size(&mut self, width: u16, height: u16)`.
   - `src/main.rs`: Re-export or import from `splash::...`, keeping `run_splash` functioning.
2. **Headless Test Harness (`src/testing/mod.rs` / `splash::testing`)**:
   - Define `TestHarness` wrapping `Terminal<TestBackend>` and `SplashApp`.
   - `TestHarness::new(width: u16, height: u16, config: HarnessConfig) -> Self`
   - `send_key`, `press_char`, `press_ctrl`
   - `inject_pty_output`
   - `resize(width, height)`
   - `render_frame() -> &Buffer`
3. **Unit & Integration Verification**:
   - Write tests in `tests/headless_harness.rs` verifying initialization with custom dimensions (80x24, 120x40), offscreen rendering without terminal window, key sending, and output injection.
   - Run `cargo test` and ensure 100% pass.
