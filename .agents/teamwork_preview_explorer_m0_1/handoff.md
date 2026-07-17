# Handoff Report — Codebase & UI Architecture Exploration

## 1. Observation

Direct code observations from `/Users/Travis/Repos/splash/src/main.rs`:

- **HarnessConfig & Argument Parsing**:
  - `src/main.rs:20-23`: Struct definition:
    ```rust
    pub struct HarnessConfig {
        pub command: String,
        pub args: Vec<String>,
    }
    ```
  - `src/main.rs:25-33`: Function signature:
    ```rust
    pub fn parse_args(args: &[String]) -> Result<HarnessConfig, String>
    ```

- **Leader State Machine & Key Handling**:
  - `src/main.rs:35-40`: State machine enum:
    ```rust
    pub enum LeaderState {
        #[default]
        Normal,
        LeaderPressed,
    }
    ```
  - `src/main.rs:42-47`: Action enum:
    ```rust
    pub enum KeyAction { None, Quit, Forward(Vec<u8>) }
    ```
  - `src/main.rs:49-52`: Active check: `pub fn is_active(&self) -> bool`
  - `src/main.rs:54-80`: Key handler: `pub fn handle_key(&mut self, key: &KeyEvent) -> KeyAction`
  - `src/main.rs:83-109`: Bytes converter: `pub fn key_event_to_bytes(key: &KeyEvent) -> Vec<u8>`

- **PTY Process Management**:
  - `src/main.rs:111-117`: Struct definition:
    ```rust
    pub struct PtyHarness {
        pub pty_pair: PtyPair,
        pub writer: Box<dyn Write + Send>,
        pub output_rx: Receiver<String>,
        pub child: Box<dyn portable_pty::Child + Send + Sync>,
    }
    ```
  - `src/main.rs:119-176`: Spawning signature:
    ```rust
    pub fn spawn(config: &HarnessConfig, rows: u16, cols: u16) -> Result<Self, String>
    ```
  - `src/main.rs:178-185`: Resizing signature: `pub fn resize(&self, rows: u16, cols: u16)`
  - `src/main.rs:187-190`: Writing signature: `pub fn write(&mut self, bytes: &[u8])`

- **Ratatui Terminal Rendering & Frame Drawing**:
  - `src/main.rs:224-228`: Initialization:
    ```rust
    enable_raw_mode().map_err(|e| e.to_string())?;
    let mut stdout = stdout();
    execute!(stdout, EnterAlternateScreen).map_err(|e| e.to_string())?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend).map_err(|e| e.to_string())?;
    ```
  - `src/main.rs:253-267`: Frame rendering loop:
    ```rust
    terminal.draw(|f| {
        let rect = f.size();
        harness.resize(rect.height, rect.width);

        let title = if leader_active {
            format!("{} [LEADER ACTIVE]", cmd_title)
        } else {
            cmd_title.clone()
        };

        let block = Block::default().title(title).borders(Borders::ALL);
        let paragraph = Paragraph::new(display_text).block(block);
        f.render_widget(paragraph, rect);
    })
    ```

- **Titles, Borders, & Leader Key Active Indicator**:
  - Border: `Block::default().borders(Borders::ALL)` (`src/main.rs:264`)
  - Title string: `let cmd_title = format!(" Harness: {} (Leader: Ctrl+B | Exit: Ctrl+B q) ", config.command);` (`src/main.rs:251`)
  - `[LEADER ACTIVE]` indicator: appended to block title when `leader_state.is_active()` returns `true` (`src/main.rs:258-262`).

- **Test Suite Execution**:
  - `cargo test` executed successfully: 6 passing tests (`test_should_exit`, `test_parse_args_valid`, `test_parse_args_with_cmd_args`, `test_parse_args_missing`, `test_leader_state_machine`, `test_pty_harness_spawn_and_read`).

---

## 2. Logic Chain

1. **Observation**: `src/main.rs` contains all existing data structures (`HarnessConfig`, `LeaderState`, `KeyAction`, `PtyHarness`) and the top-level loop `run_splash`.
2. **Reasoning**: Currently, `run_splash` handles state management, event loop processing, and rendering in a single inline function without formal abstractions for `App`, `Tab`, `FileTree`, `MainPane`, or `StatusBar`.
3. **Observation**: `docs/spec/poc.md` defines a layout with Tab Bar (top), File Tree (left 20%), Main Pane (right 80%), and specific keybindings for tab/tree focus.
4. **Reasoning**: The codebase is in a prototype state where terminal rendering, PTY integration, and `Ctrl+B` leader handling work end-to-end, but needs refactoring into explicit modules (`app`, `ui`, `tab`, `file_tree`, `terminal`) to support full POC features.
5. **Observation**: The `[LEADER ACTIVE]` indicator is dynamically constructed inside the frame render closure (`terminal.draw`) by checking `leader_state.is_active()`.
6. **Reasoning**: The rendering flow directly relies on `crossterm` raw mode + `ratatui::Terminal::draw` taking a `Frame` closure.

---

## 3. Caveats

- **No Code Modifications Made**: Per investigation scope and constraints, no modifications were written to `src/` or `tests/`.
- **Existing Scope vs Spec Scope**: The current code in `src/main.rs` does not yet contain code for rendering tabs or a file tree; those exist in `docs/spec/poc.md` and `CONTEXT.md` as specifications to be implemented in phase/milestone work.

---

## 4. Conclusion

Splash's UI architecture relies on Ratatui (`0.26`) on top of Crossterm (`0.27`) with `portable-pty` (`0.8`). The codebase is completely self-contained within `src/main.rs`. All rendering is driven synchronously inside `terminal.draw` during the main event loop. The `[LEADER ACTIVE]` state is rendered dynamically as a block title suffix whenever `LeaderState::LeaderPressed` is active.

---

## 5. Verification Method

To independently verify the observations and analysis:
1. Inspect `src/main.rs` at the exact line numbers quoted in Section 1 using `view_file`.
2. Run `cargo test` in `/Users/Travis/Repos/splash` to confirm the 6 existing tests compile and pass.
3. Review `analysis.md` in `/Users/Travis/Repos/splash/.agents/teamwork_preview_explorer_m0_1/analysis.md`.
