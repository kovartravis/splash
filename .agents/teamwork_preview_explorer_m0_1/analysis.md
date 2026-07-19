# Splash Codebase & UI Architecture Analysis

## Executive Summary

Splash is an agent harness shell built with **Rust**, **Ratatui** (`0.26`), **crossterm** (`0.27`), and **portable-pty** (`0.8`).
Currently, the codebase contains a monolithic prototype implementation in `src/main.rs` (386 lines) that demonstrates basic terminal initialization, `portable-pty` process spawning, `crossterm` event polling, `Ctrl+B` leader key state management, and basic Ratatui `Paragraph` block rendering.

This report documents:
1. Exact file paths, data structures, function signatures, and line numbers of the current implementation.
2. How Ratatui `Terminal` and widgets are rendered.
3. The location and mechanics of titles, borders, status hints, and active indicators (`[LEADER ACTIVE]`).
4. Alignment and gaps between the current `src/main.rs` and the specified architecture in `docs/spec/poc.md` & `CONTEXT.md`.

---

## 1. Data Structures & Functions Map

All application logic currently resides in `src/main.rs`.

| Struct / Enum / Function | Line Numbers | Description & Signature |
|---|---|---|
| `HarnessConfig` | `src/main.rs:20-23` | Configuration struct holding harness CLI binary and arguments:<br>`pub struct HarnessConfig { pub command: String, pub args: Vec<String> }` |
| `parse_args` | `src/main.rs:25-33` | CLI argument parser:<br>`pub fn parse_args(args: &[String]) -> Result<HarnessConfig, String>` |
| `LeaderState` | `src/main.rs:35-40` | State machine enum for `Ctrl+B` leader key:<br>`pub enum LeaderState { Normal, LeaderPressed }` |
| `LeaderState::is_active` | `src/main.rs:49-52` | Returns `true` if `LeaderPressed` state is active:<br>`pub fn is_active(&self) -> bool` |
| `LeaderState::handle_key` | `src/main.rs:54-80` | Handles input events; transitions state and yields action:<br>`pub fn handle_key(&mut self, key: &KeyEvent) -> KeyAction` |
| `KeyAction` | `src/main.rs:42-47` | Action output from key handling:<br>`pub enum KeyAction { None, Quit, Forward(Vec<u8>) }` |
| `key_event_to_bytes` | `src/main.rs:83-109` | Converts crossterm `KeyEvent` into byte sequences for PTY input:<br>`pub fn key_event_to_bytes(key: &KeyEvent) -> Vec<u8>` |
| `PtyHarness` | `src/main.rs:111-117` | Encapsulates background PTY pair, writer stream, child process, and output thread channel:<br>`pub struct PtyHarness { pub pty_pair: PtyPair, pub writer: Box<dyn Write + Send>, pub output_rx: Receiver<String>, pub child: Box<...> }` |
| `PtyHarness::spawn` | `src/main.rs:119-176` | Spawns harness command in `portable_pty` and starts output reader thread:<br>`pub fn spawn(config: &HarnessConfig, rows: u16, cols: u16) -> Result<Self, String>` |
| `PtyHarness::resize` | `src/main.rs:178-185` | Resizes master PTY window dimensions:<br>`pub fn resize(&self, rows: u16, cols: u16)` |
| `PtyHarness::write` | `src/main.rs:187-190` | Flushes input bytes into PTY writer:<br>`pub fn write(&mut self, bytes: &[u8])` |
| `run_splash` | `src/main.rs:223-285` | Main application loop; initializes terminal, spawns PTY, handles draw calls and key events:<br>`fn run_splash(config: HarnessConfig) -> Result<(), String>` |
| `restore_terminal` | `src/main.rs:217-221` | Disables terminal raw mode and leaves alternate screen:<br>`fn restore_terminal() -> io::Result<()>` |
| `should_exit` | `src/main.rs:287-293` | Test helper checking quit keystrokes:<br>`fn should_exit(key: &KeyEvent) -> bool` |

---

## 2. Ratatui Terminal & Rendering Architecture

### Terminal Setup & Teardown
- **Initialization** (`src/main.rs:224-228`):
  ```rust
  enable_raw_mode().map_err(|e| e.to_string())?;
  let mut stdout = stdout();
  execute!(stdout, EnterAlternateScreen).map_err(|e| e.to_string())?;
  let backend = CrosstermBackend::new(stdout);
  let mut terminal = Terminal::new(backend).map_err(|e| e.to_string())?;
  ```
- **Cleanup / Panic Hook** (`src/main.rs:205-208`, `217-221`):
  Uses `std::panic::set_hook` to ensure terminal state is restored on panic via `restore_terminal()`.

### Frame Drawing & Render Loop
- **Draw Call** (`src/main.rs:253-267`):
  The render closure `terminal.draw(|f| { ... })` is called every frame iteration (polling interval ~30ms):
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
- **Widgets Used**:
  - `ratatui::widgets::Block` (`src/main.rs:264`): Container border with title.
  - `ratatui::widgets::Paragraph` (`src/main.rs:265`): Displays bounded PTY terminal string (`display_text`, lines capped to last 100 lines, `src/main.rs:244-248`).

---

## 3. Title, Borders, Status Bar, & Active Indicators

- **Borders**: Rendered around full screen `rect` via `Borders::ALL` (`src/main.rs:264`).
- **Command & Key Hint Title**: Built at `src/main.rs:251`:
  ```rust
  let cmd_title = format!(" Harness: {} (Leader: Ctrl+B | Exit: Ctrl+B q) ", config.command);
  ```
- **Leader Key Indicator `[LEADER ACTIVE]`**:
  - State check: `let leader_active = leader_state.is_active();` (`src/main.rs:250`)
  - Title formatting (`src/main.rs:258-262`):
    ```rust
    let title = if leader_active {
        format!("{} [LEADER ACTIVE]", cmd_title)
    } else {
        cmd_title.clone()
    };
    ```
- **Status Bar**: Dedicated status bar widget does not exist yet; status information is embedded directly in the block title.

---

## 4. Architectural Gaps & Refactoring Roadmap (POC Spec vs Codebase)

Per `docs/spec/poc.md` and `CONTEXT.md`, Splash's target UI architecture requires the following components:

1. **Tab Bar** (Top, full width):
   - Needs `TabBar` widget / state struct.
   - Requires tab data structures (`Tab::Harness(HarnessTab)`, `Tab::File(FileTab)`).
2. **File Tree** (Left sidebar, ~20% width):
   - Needs `FileTree` state struct (directory node tree, expand/collapse state, cursor position).
   - Needs tree navigation key handling (`↑`, `↓`, `←`, `→`, `Enter`).
3. **Main Pane** (Right area, ~80% width):
   - Hosts active harness terminal OR active file tab viewer.
   - Needs layout split (`ratatui::layout::Layout::default().direction(Direction::Horizontal).constraints(...)`).
4. **App State**:
   - Extraction of `run_splash` inline variables into an `App` struct holding tab list, active tab index, focus state (Tree vs Main Pane), file tree state, and leader state.

---

## 5. Verification & Test Suite

The existing unit tests in `src/main.rs` (`lines 295-385`) cover:
- `test_should_exit` (`src/main.rs:299-314`)
- `test_parse_args_valid`, `test_parse_args_with_cmd_args`, `test_parse_args_missing` (`src/main.rs:317-343`)
- `test_leader_state_machine` (`src/main.rs:346-361`)
- `test_pty_harness_spawn_and_read` (`src/main.rs:364-383`)

All 6 unit tests run cleanly via `cargo test`.
