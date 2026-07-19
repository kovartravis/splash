# Handoff Report: Splash Event Loop, PTY Streams, Keyboard Input, and Terminal Resizing

## 1. Observation

Direct evidence collected from code inspection of `/Users/Travis/Repos/splash/src/main.rs` and `/Users/Travis/Repos/splash/Cargo.toml`:

- **Dependencies**: `Cargo.toml` specifies `ratatui = "0.26"`, `crossterm = { version = "0.27", features = ["event-stream"] }`, and `portable-pty = "0.8"`.
- **Event Loop Location**: `run_splash` function in `src/main.rs` (lines 223–285). The main loop construct is `loop { ... }` (lines 236–281).
- **Polling Mechanism**: `crossterm::event::poll(Duration::from_millis(30))` (`src/main.rs:270`).
- **Input Processing**: `event::read()` (`src/main.rs:271`) returning `crossterm::event::Event::Key(key)`.
- **Leader Key State Machine**:
  - `LeaderState` enum (`src/main.rs:35–40`) with `Normal` and `LeaderPressed` variants.
  - `KeyAction` enum (`src/main.rs:42–47`) with `None`, `Quit`, and `Forward(Vec<u8>)` variants.
  - `LeaderState::handle_key` (`src/main.rs:54–80`) handles state transitions:
    - `Normal` state + `Ctrl+B` -> switches to `LeaderPressed`, returns `KeyAction::None`.
    - `Normal` state + character key -> converts key to bytes via `key_event_to_bytes` (`src/main.rs:83–109`), returns `KeyAction::Forward(bytes)`.
    - `LeaderPressed` state + `q`/`Q` -> returns `KeyAction::Quit`.
    - `LeaderPressed` state + `Ctrl+B` -> returns `KeyAction::Forward(vec![0x02])` (escaped pass-through).
    - `LeaderPressed` state + unhandled key -> returns `KeyAction::None`.
- **PTY Channels & Background Thread**:
  - PTY master reader cloned and passed to `std::thread::spawn` worker thread (`src/main.rs:154–168`).
  - Reads in 1024-byte chunks (`reader.read(&mut buf)`), converts via `String::from_utf8_lossy(&buf[..n])`, and sends to `std::sync::mpsc::channel()` (`src/main.rs:152, 160`).
  - Drained in main event loop via non-blocking `harness.output_rx.try_recv()` (`src/main.rs:238`).
  - Written to PTY writer via `harness.write(&bytes)` (`src/main.rs:187–190, 275`), which invokes `writer.write_all(bytes)` and `writer.flush()`.
- **Terminal Resizing**:
  - On every frame inside `terminal.draw(|f| { ... })` (`src/main.rs:254–256`), `rect = f.size()` is retrieved and `harness.resize(rect.height, rect.width)` is called.
  - `PtyHarness::resize` (`src/main.rs:178–185`) resizes PTY master using `portable_pty::MasterPty::resize(PtySize { rows, cols, ... })`.

---

## 2. Logic Chain

1. **Input Interception**: Splash uses a modal state machine (`LeaderState`) to ensure Splash control chords (like `Ctrl+B q` to exit) do not pollute or conflict with child PTY processes.
2. **Keyboard Translation**: When in `Normal` state, keyboard input captured by `crossterm::event::read()` is converted into raw UTF-8 or ASCII control byte arrays by `key_event_to_bytes`. These bytes are flushed directly to the PTY master's `Write` implementation via `harness.write(&bytes)`.
3. **PTY Async-to-Sync Bridge**: Because PTY reading is a blocking I/O operation, `PtyHarness::spawn` spawns a background OS thread that reads up to 1024 bytes at a time and pushes lossy UTF-8 strings into an `mpsc::channel`.
4. **Render Loop Synergy**: The main thread loop drains `output_rx` without blocking (`try_recv()`), updates the string buffer (`raw_output`), formats the latest lines, and renders them to Ratatui's `Paragraph` widget.
5. **Dynamic Window Resizing**: By calling `harness.resize(rect.height, rect.width)` inside `terminal.draw`, Splash dynamically synchronizes the child process PTY size with any terminal window resize detected by Crossterm/Ratatui, triggering `SIGWINCH` in the slave PTY process.

---

## 3. Caveats

- **Read-only Investigation**: No source code under `src/` or `tests/` was modified during this exploration.
- **POC Stream Model**: The current implementation treats PTY output as a growing text stream rendered in a `Paragraph` widget with line truncation (last 100 lines). Complete ANSI/VT100 escape sequence parsing and screen cell grid buffering (e.g. via `vt100` crate or custom terminal emulator state) will be required as Splash evolves beyond the POC stage.
- **Leader Key Swallow**: Any unrecognized key pressed immediately after `Ctrl+B` while in `LeaderPressed` state is currently swallowed (`KeyAction::None`) and resets `LeaderState` back to `Normal`.

---

## 4. Conclusion

Splash's m0_2 POC architecture cleanly encapsulates PTY lifecycle, keyboard event state management, thread-safe output streaming, and terminal size synchronization within `src/main.rs`. Key events are filtered through `LeaderState::handle_key`, PTY output flows from a background thread via `mpsc::channel` to `try_recv()`, and PTY window dimensions are updated on each render tick matching Ratatui's frame rect.

---

## 5. Verification Method

To independently verify the observations and analysis:

1. **Codebase Inspection**:
   - `view_file` on `src/main.rs` lines 35–109 to verify `LeaderState`, `KeyAction`, and `key_event_to_bytes`.
   - `view_file` on `src/main.rs` lines 111–191 to verify `PtyHarness::spawn`, thread reader, channel receiver, `resize`, and `write`.
   - `view_file` on `src/main.rs` lines 223–285 to verify `run_splash`, `output_rx.try_recv()`, `terminal.draw`, `harness.resize`, and `event::poll`.

2. **Test Suite Verification**:
   - Run `cargo test` to execute unit tests verifying `LeaderState` transitions (`test_leader_state_machine`) and `PtyHarness` spawning/reading (`test_pty_harness_spawn_and_read`).

3. **Invalidation Conditions**:
   - The analysis would be invalidated if `PtyHarness` is converted to an async runtime (e.g. `tokio`), if `crossterm::event::poll` is replaced by an async event stream loop, or if `LeaderState` is replaced by raw Crossterm key matchers.
