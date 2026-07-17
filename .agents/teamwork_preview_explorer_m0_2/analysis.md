# Analysis of Event Loop, PTY Streams, Keyboard Input, and Terminal Resize in Splash

## Executive Summary

Splash is an agent harness shell built in Rust using `ratatui` (0.26), `crossterm` (0.27 with `event-stream`), and `portable-pty` (0.8). Its current implementation resides in `src/main.rs`. Event processing relies on a synchronous event loop polling `crossterm::event::poll(Duration::from_millis(30))` coupled with a `std::sync::mpsc` channel for receiving asynchronous PTY output chunks from a dedicated background thread. Leader key state management (`Ctrl+B`) is implemented via a finite state machine (`LeaderState`).

---

## Architecture & Component Overview

| Component | File Path | Line Range | Responsibilities |
|---|---|---|---|
| `run_splash` | `src/main.rs` | `223–285` | Core event loop, terminal draw calls, Crossterm polling |
| `LeaderState` | `src/main.rs` | `35–81` | Finite state machine for `Ctrl+B` leader key transitions |
| `KeyAction` | `src/main.rs` | `42–47` | Action output from key handling (`None`, `Quit`, `Forward`) |
| `key_event_to_bytes` | `src/main.rs` | `83–109` | Converts Crossterm `KeyEvent` into byte sequences for PTY input |
| `PtyHarness` | `src/main.rs` | `111–191` | Encapsulates `PtyPair`, writer, child process, output channel receiver, and resize/write operations |

---

## 1. Event Loop & Dispatch Mechanism

### Loop Structure
The main loop runs synchronously inside `run_splash()` (`src/main.rs:236–281`).

```rust
loop {
    // 1. Drain incoming PTY output from channel
    while let Ok(chunk) = harness.output_rx.try_recv() {
        raw_output.push_str(&chunk);
    }

    // 2. Format / trim output text for display
    let lines: Vec<&str> = raw_output.lines().collect();
    let display_text = if lines.len() > 100 {
        lines[lines.len() - 100..].join("\n")
    } else {
        raw_output.clone()
    };

    // 3. Render frame to terminal
    terminal.draw(|f| {
        let rect = f.size();
        harness.resize(rect.height, rect.width);
        ...
        f.render_widget(paragraph, rect);
    })?;

    // 4. Poll keyboard input with 30ms timeout
    if event::poll(Duration::from_millis(30))? {
        if let Event::Key(key) = event::read()? {
            match leader_state.handle_key(&key) {
                KeyAction::Quit => break,
                KeyAction::Forward(bytes) => harness.write(&bytes),
                KeyAction::None => {}
            }
        }
    }
}
```

### Channel Mechanisms
- **Type**: `std::sync::mpsc::channel::<String>()` (`src/main.rs:152`)
- **Producer**: Spawned background reader thread (`src/main.rs:154–168`)
- **Consumer**: `harness.output_rx.try_recv()` inside the main event loop (`src/main.rs:238`)

---

## 2. Keyboard Event Processing & Leader Key State Machine

### Finite State Machine (`LeaderState`)
`LeaderState` (`src/main.rs:35–81`) manages modal key handling to prevent harness processes from intercepting Splash control chords.

```rust
pub enum LeaderState {
    Normal,
    LeaderPressed,
}
```

#### State Transition Logic (`handle_key`):

1. **State: `LeaderState::Normal`**
   - **`Ctrl+B` pressed** (`key.code == KeyCode::Char('b') && key.modifiers.contains(KeyModifiers::CONTROL)`):
     - Transitions state to `LeaderState::LeaderPressed`.
     - Returns `KeyAction::None` (intercepts key, does NOT pass `Ctrl+B` to PTY).
   - **Any other key**:
     - Calls `key_event_to_bytes(key)`.
     - Returns `KeyAction::Forward(bytes)` (or `KeyAction::None` if empty).

2. **State: `LeaderState::LeaderPressed`**
   - **State reset**: Immediately transitions back to `LeaderState::Normal` regardless of key pressed.
   - **`q` or `Q` pressed**:
     - Returns `KeyAction::Quit` -> main loop breaks and Splash exits cleanly.
   - **`Ctrl+B` pressed** (`KeyCode::Char('b')` with `KeyModifiers::CONTROL`):
     - Returns `KeyAction::Forward(vec![0x02])` (ASCII STX byte for `Ctrl+B`), allowing escape pass-through of `Ctrl+B` to the child PTY process.
   - **Any other key**:
     - Returns `KeyAction::None` (swallows unhandled key).

### Key Byte Conversion (`key_event_to_bytes`)
Located at `src/main.rs:83–109`:
- **Control Modifiers (`Ctrl+a` .. `Ctrl+z`)**: Converted to ASCII 1..26 (`(lower as u8) - b'a' + 1`).
- **Standard Characters (`KeyCode::Char(c)`)**: Encoded into UTF-8 byte array (`c.encode_utf8(&mut buf).as_bytes().to_vec()`).
- **Special Keys**:
  - `KeyCode::Enter` -> `vec![b'\r']`
  - `KeyCode::Backspace` -> `vec![0x7f]`
  - `KeyCode::Tab` -> `vec![b'\t']`
  - `KeyCode::Esc` -> `vec![0x1b]`

---

## 3. PTY Input & Output Stream Handling

### PTY Spawning & Infrastructure (`PtyHarness::spawn`)
Located at `src/main.rs:119–176`:
- Uses `portable_pty::native_pty_system()`.
- Opens PTY pair with `pty_system.openpty(PtySize { rows, cols, pixel_width: 0, pixel_height: 0 })`.
- Spawns command (`CommandBuilder::new(&config.command)`) on slave side (`pty_pair.slave.spawn_command(cmd)`).
- Takes master writer (`pty_pair.master.take_writer()`) and reader (`pty_pair.master.try_clone_reader()`).

### Output Stream (Child -> Splash TUI)
- **Thread**: `std::thread::spawn` background worker thread (`src/main.rs:154–168`).
- **Buffer**: Fixed 1024-byte read buffer (`let mut buf = [0u8; 1024]`).
- **Read loop**: Synchronous blocking `reader.read(&mut buf)`.
- **Lossy UTF-8 conversion**: `String::from_utf8_lossy(&buf[..n]).to_string()`.
- **Channel transmission**: `tx.send(text)`. Thread terminates cleanly on EOF (`Ok(0)`), read error, or channel disconnection.
- **Main Loop consumption**: Non-blocking `output_rx.try_recv()` drains channel into `raw_output` string buffer (`src/main.rs:238`).
- **Rendering**: Display buffer bounded to last 100 lines and rendered into Ratatui `Paragraph` widget (`src/main.rs:244–266`).

### Input Stream (User Keyboard -> Child PTY)
- When `LeaderState::handle_key` returns `KeyAction::Forward(bytes)`, main loop calls `harness.write(&bytes)` (`src/main.rs:275`).
- `PtyHarness::write` (`src/main.rs:187–190`) calls `self.writer.write_all(bytes)` followed by `self.writer.flush()`.

---

## 4. Terminal Resize Handling

### Mechanics
1. **Initial Size Setup**:
   - `run_splash` queries terminal dimensions from `terminal.size()` (`src/main.rs:230`).
   - `PtyHarness::spawn` initializes PTY with these initial dimensions (`src/main.rs:123–128`).

2. **Dynamic Resize on Render Frame**:
   - In `terminal.draw(|f| { ... })` (`src/main.rs:254–256`), `rect` is queried via `f.size()`.
   - `harness.resize(rect.height, rect.width)` is called on every render pass.

3. **PTY Master Size Update**:
   - `PtyHarness::resize` (`src/main.rs:178–185`) executes:
     ```rust
     self.pty_pair.master.resize(PtySize {
         rows,
         cols,
         pixel_width: 0,
         pixel_height: 0,
     })
     ```
   - On Unix operating systems, `portable-pty` issues an `ioctl(TIOCSWINSZ)` on the master PTY file descriptor, which sends a `SIGWINCH` signal to the child process (e.g. 80x24, 120x40), triggering immediate layout recalculation in the underlying shell/harness.
