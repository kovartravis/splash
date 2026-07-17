# Splash Test Harness & Snapshot Design Analysis

## Executive Summary

This report delivers the architectural design and concrete API proposals for Splash's headless test harness, visual buffer snapshot infrastructure, and end-to-end (E2E) integration test helpers.

Splash relies on `ratatui = "0.26"`, `crossterm = "0.27"`, and `portable-pty = "0.8"` (as defined in `Cargo.toml:6-10`). `ratatui` includes builtin `TestBackend` and `Buffer` types that allow full UI rendering and verification entirely in-memory without initializing raw terminal mode or allocating an OS TTY.

To achieve complete testability, Splash's main event loop in `src/main.rs:223-285` (currently hardcoded to `CrosstermBackend`) must be decoupled into a state container (`SplashApp` / `App`) parameterized over `ratatui::backend::Backend`. With this refactoring, both real TUI execution and headless integration tests share identical rendering and state-transition logic.

---

## 1. Ratatui `TestBackend` & `Buffer` Compatibility Analysis

### 1.1 Package Dependencies (`Cargo.toml`)
- **`ratatui`**: Version `0.26.0` (`Cargo.toml:7`).
- **`ratatui::backend::TestBackend`**: Available out of the box in `ratatui::backend`.
  - Instantiation: `TestBackend::new(width, height)` creates a backend initialized with a grid of cells.
  - Resizing: `backend.resize(width, height)` updates buffer dimensions.
  - Access: `backend.buffer()` returns `&Buffer`.
- **`ratatui::buffer::Buffer`**:
  - `buffer.area()` returns `Rect { x, y, width, height }`.
  - `buffer.get(x, y)` returns `&Cell`.
  - `cell.symbol()` returns `&str` (UTF-8 character or multi-width symbol).

### 1.2 Required Dev-Dependencies
To support snapshots and regex assertions cleanly, we recommend adding the following to `Cargo.toml`:
```toml
[dev-dependencies]
pretty_assertions = "1.4"
regex = "1.10"
```

---

## 2. Architecture Decoupling for Testability

Currently, `src/main.rs:223-285` couples terminal setup, PTY polling, rendering, and event handling inside `run_splash`:

```rust
// Existing src/main.rs:227-228
let backend = CrosstermBackend::new(stdout);
let mut terminal = Terminal::new(backend).map_err(|e| e.to_string())?;
```

### Proposed Refactoring: `SplashApp` & Generic Rendering
Extract UI state into `SplashApp` and parameterize rendering:

```rust
pub struct SplashApp {
    pub config: HarnessConfig,
    pub leader_state: LeaderState,
    pub raw_output: String,
    // Future state: active_tab, file_tree_state, focus_pane, etc.
}

impl SplashApp {
    pub fn new(config: HarnessConfig) -> Self { ... }
    
    pub fn render<B: Backend>(&mut self, frame: &mut Frame<B>, harness_size: (u16, u16)) {
        let rect = frame.size();
        let cmd_title = format!(" Harness: {} (Leader: Ctrl+B | Exit: Ctrl+B q) ", self.config.command);
        let title = if self.leader_state.is_active() {
            format!("{} [LEADER ACTIVE]", cmd_title)
        } else {
            cmd_title
        };
        let block = Block::default().title(title).borders(Borders::ALL);
        let paragraph = Paragraph::new(self.display_text()).block(block);
        frame.render_widget(paragraph, rect);
    }
    
    pub fn handle_key(&mut self, key: &KeyEvent) -> KeyAction {
        self.leader_state.handle_key(key)
    }

    pub fn push_pty_output(&mut self, chunk: &str) {
        self.raw_output.push_str(chunk);
    }
}
```

This allows `TestBackend` to render `SplashApp` using identical frame drawing functions as `CrosstermBackend`.

---

## 3. Headless Test Harness Abstraction (`TestHarness`)

The headless test harness wraps `Terminal<TestBackend>` and provides a fluently testable driver for unit and integration tests.

### 3.1 `TestHarness` Struct & API Proposal

```rust
pub struct TestHarness {
    pub terminal: Terminal<TestBackend>,
    pub app: SplashApp,
    pub mock_pty: MockPty,
}

impl TestHarness {
    /// Create a new headless test harness with default size (80x24)
    pub fn new(config: HarnessConfig) -> Self {
        Self::with_size(config, 80, 24)
    }

    /// Create a headless test harness with explicit dimensions
    pub fn with_size(config: HarnessConfig, width: u16, height: u16) -> Self {
        let backend = TestBackend::new(width, height);
        let terminal = Terminal::new(backend).expect("Failed to initialize TestBackend");
        let app = SplashApp::new(config);
        let mock_pty = MockPty::new();
        
        let mut harness = Self { terminal, app, mock_pty };
        harness.render();
        harness
    }

    /// Resize the virtual terminal
    pub fn resize(&mut self, width: u16, height: u16) {
        self.terminal.backend_mut().resize(width, height);
        self.render();
    }

    /// Render the current app state to the TestBackend buffer
    pub fn render(&mut self) {
        let size = self.terminal.size().unwrap();
        let (width, height) = (size.width, size.height);
        self.app.render(&mut self.terminal.get_frame(), (width, height));
    }

    /// Inject a Crossterm KeyEvent into the app
    pub fn send_key(&mut self, key: KeyEvent) -> KeyAction {
        let action = self.app.handle_key(&key);
        if let KeyAction::Forward(ref bytes) = action {
            self.mock_pty.write_input(bytes);
        }
        self.render();
        action
    }

    /// Convenience helper: press a char key without modifiers
    pub fn press_char(&mut self, c: char) -> KeyAction {
        self.send_key(KeyEvent::new(KeyCode::Char(c), KeyModifiers::empty()))
    }

    /// Convenience helper: press Ctrl+Key (e.g. Ctrl+B leader key)
    pub fn press_ctrl(&mut self, c: char) -> KeyAction {
        self.send_key(KeyEvent::new(KeyCode::Char(c), KeyModifiers::CONTROL))
    }

    /// Inject simulated output chunk from PTY stream
    pub fn inject_pty_output(&mut self, text: &str) {
        self.app.push_pty_output(text);
        self.render();
    }

    /// Get current rendered buffer reference
    pub fn buffer(&self) -> &Buffer {
        self.terminal.backend().buffer()
    }

    /// Export buffer as a formatted ASCII grid with outer borders
    pub fn snapshot_string(&self) -> String {
        format_buffer_grid(self.buffer())
    }
}
```

---

## 4. Visual Buffer Snapshot Helpers & Assertion Format

### 4.1 Grid Formatting Specification
To make snapshot assertions human-readable in test failures and git diffs, the buffer grid is formatted with surrounding box borders (`┌`, `─`, `┐`, `│`, `└`, `┘`).

#### Formatting Function (`format_buffer_grid`):
```rust
pub fn format_buffer_grid(buffer: &Buffer) -> String {
    let area = buffer.area();
    let width = area.width as usize;
    let height = area.height as usize;
    let mut lines = Vec::with_capacity(height + 2);

    // Top Border
    lines.push(format!("┌{}┐", "─".repeat(width)));

    // Content Rows
    for y in 0..height {
        let mut row_str = String::with_capacity(width);
        for x in 0..width {
            let cell = buffer.get(x as u16, y as u16);
            row_str.push_str(cell.symbol());
        }
        lines.push(format!("│{}│", row_str));
    }

    // Bottom Border
    lines.push(format!("└{}┘", "─".repeat(width)));

    lines.join("\n")
}
```

### 4.2 Snapshot Assertion Macros & Functions

```rust
/// Assert exact grid snapshot match
#[macro_export]
macro_rules! assert_snapshot {
    ($harness:expr, $expected:expr) => {
        let actual = $harness.snapshot_string();
        let expected_trimmed = $expected.trim_matches('\n');
        pretty_assertions::assert_eq!(
            actual,
            expected_trimmed,
            "Visual buffer snapshot mismatch!"
        );
    };
}

/// Assert screen buffer contains substring
pub fn assert_buffer_contains(harness: &TestHarness, expected_text: &str) {
    let snapshot = harness.snapshot_string();
    assert!(
        snapshot.contains(expected_text),
        "Expected screen buffer to contain {:?}, but was:\n{}",
        expected_text,
        snapshot
    );
}

/// Assert screen buffer matches regex pattern
pub fn assert_buffer_matches(harness: &TestHarness, regex_pattern: &str) {
    let snapshot = harness.snapshot_string();
    let re = regex::Regex::new(regex_pattern).expect("Invalid regex pattern in assertion");
    assert!(
        re.is_match(&snapshot),
        "Expected screen buffer to match regex {:?}, but was:\n{}",
        regex_pattern,
        snapshot
    );
}
```

### 4.3 Example Snapshot Format
For a 40x8 test screen:
```
┌────────────────────────────────────────┐
│ Harness: echo (Leader: Ctrl+B | Exit..│
│                                        │
│ hello_splash                           │
│                                        │
│                                        │
│                                        │
│                                        │
└────────────────────────────────────────┘
```

---

## 5. End-to-End Integration Test Helpers

### 5.1 `MockPty` Stream Simulation
For unit and fast integration tests, `MockPty` simulates PTY I/O in memory without spawning child processes:

```rust
pub struct MockPty {
    pub written_bytes: Vec<u8>,
}

impl MockPty {
    pub fn new() -> Self {
        Self { written_bytes: Vec::new() }
    }

    pub fn write_input(&mut self, bytes: &[u8]) {
        self.written_bytes.extend_from_slice(bytes);
    }

    pub fn drained_input(&mut self) -> Vec<u8> {
        std::mem::take(&mut self.written_bytes)
    }
}
```

### 5.2 Real PTY Integration Driver (`RealPtyHarnessTest`)
For full E2E testing of `PtyHarness` (wrapping `portable-pty`), tests run real process commands (e.g., `echo`, `sh`) while capturing terminal frames in `TestBackend`:

```rust
pub struct RealPtyHarnessTest {
    pub harness: PtyHarness,
    pub terminal: Terminal<TestBackend>,
    pub app: SplashApp,
}

impl RealPtyHarnessTest {
    pub fn spawn_real(config: &HarnessConfig, width: u16, height: u16) -> Self {
        let harness = PtyHarness::spawn(config, height, width)
            .expect("Failed to spawn real PTY harness");
        let backend = TestBackend::new(width, height);
        let terminal = Terminal::new(backend).unwrap();
        let app = SplashApp::new(config.clone());

        Self { harness, terminal, app }
    }

    /// Drain pending PTY receiver output within a timeout
    pub fn pump_pty_output(&mut self, timeout: std::time::Duration) -> bool {
        let start = std::time::Instant::now();
        let mut got_output = false;
        while start.elapsed() < timeout {
            while let Ok(chunk) = self.harness.output_rx.try_recv() {
                self.app.push_pty_output(&chunk);
                got_output = true;
            }
            if got_output {
                break;
            }
            std::thread::sleep(std::time::Duration::from_millis(10));
        }
        let size = self.terminal.size().unwrap();
        self.app.render(&mut self.terminal.get_frame(), (size.width, size.height));
        got_output
    }
}
```

---

## 6. Recommended Test Module & Directory Layout

To maintain layout compliance and clear isolation, we propose the following structure:

```
splash/
├── src/
├── tests/
│   ├── common/
│   │   ├── mod.rs           # Re-exports test harness & snapshot assertions
│   │   ├── harness.rs       # TestHarness & MockPty implementation
│   │   └── snapshots.rs     # format_buffer_grid, assert_snapshot!, regex helpers
│   ├── leader_keys.rs       # Integration test: Ctrl+B key sequences
│   ├── pty_integration.rs   # Integration test: real PTY output & resize handling
│   └── visual_layout.rs     # Snapshot tests: tab bar, leader active, file tree
```

---

## 7. Verification Method

To verify the test harness infrastructure once implemented:
1. Run `cargo test` to execute unit and snapshot integration tests.
2. Run `cargo test --test visual_layout` to verify visual snapshot assertions.
3. Inspect mismatch diffs generated by `pretty_assertions` when intentionally breaking a snapshot.
