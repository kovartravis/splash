# Handoff Report — Challenger 1 (Milestone 1)

## 1. Observation

Empirical testing was performed using a newly constructed stress test suite in `tests/stress_tests.rs` alongside existing tests via `cargo test`.

Key tool execution command and result:
```
$ cargo test --test stress_tests -- --nocapture
running 13 tests
test test_key_event_to_bytes_unmapped_keys ... ok
test test_leader_state_swallows_unrecognized_keys ... ok
test test_ansi_escape_sequences_and_control_chars ... ok
test test_large_single_line_output ... ok
test test_rapid_leader_quit_sequence ... ok
test test_utf8_split_boundary_in_pty_buffer ... ok
test test_rapid_leader_toggling ... ok
10,000 rapid keypresses took 3.513583ms
10,000 lines push: 21.5µs, render: 4.479916ms
test test_rapid_key_sequences ... ok
test test_large_pty_output_chunks_10000_lines ... ok
Render time with 50000 lines (raw_output len: 538890 bytes): 4.169708ms
Render time with 100000 lines (raw_output len: 1077780 bytes): 7.850292ms
100,000 lines push: 139.458µs, render: 7.940042ms
test test_large_pty_output_chunks_100000_lines ... ok
Render time with 150000 lines (raw_output len: 1616670 bytes): 11.500166ms
Render time with 200000 lines (raw_output len: 2155560 bytes): 16.114333ms
Render time with 250000 lines (raw_output len: 2694450 bytes): 18.098583ms
test test_raw_output_performance_degradation_benchmark ... ok
500 heavy interleaved cycles took 615.053667ms
test test_interleaved_heavy_workload ... ok
1000 rapid resizes took 1.567252875s
test test_rapid_resizes_and_extreme_dimensions ... ok
test result: ok. 13 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 1.63s
```

Specific code observations:

- **src/app.rs (lines 28–34)**:
```rust
let lines: Vec<&str> = self.raw_output.lines().collect();
let display_text = if lines.len() > 100 {
    lines[lines.len() - 100..].join("\n")
} else {
    self.raw_output.clone()
};
```

- **src/pty.rs (lines 67–81)**:
```rust
thread::spawn(move || {
    let mut buf = [0u8; 1024];
    loop {
        match reader.read(&mut buf) {
            Ok(0) => break,
            Ok(n) => {
                let text = String::from_utf8_lossy(&buf[..n]).to_string();
                if tx.send(text).is_err() {
                    break;
                }
            }
            Err(_) => break,
        }
    }
});
```

- **src/leader.rs (lines 37–46)**:
```rust
LeaderState::LeaderPressed => {
    *self = LeaderState::Normal;
    match key.code {
        KeyCode::Char('q') | KeyCode::Char('Q') => KeyAction::Quit,
        KeyCode::Char('b') if key.modifiers.contains(KeyModifiers::CONTROL) => {
            KeyAction::Forward(vec![0x02])
        }
        _ => KeyAction::None,
    }
}
```

- **src/leader.rs (lines 51–77)**:
`key_event_to_bytes` returns `vec![]` for `KeyCode::Up`, `KeyCode::Down`, `KeyCode::Left`, `KeyCode::Right`, `KeyCode::F(_)`, `KeyCode::Home`, `KeyCode::End`, `KeyCode::PageUp`, `KeyCode::PageDown`, `KeyCode::Delete`, `KeyCode::Insert`, `Ctrl+[`, and any `Alt` modifiers.

- **tests/stress_tests.rs (lines 35–39)**:
Resizing to `(500, 500)` yields `app.terminal_size == (500, 500)` while `buffer.area.width == 255` and `buffer.area.height == 255`.

## 2. Logic Chain

1. **Rapid Resizes**:
   - Resizing 1,000 times across extreme bounds (`(1,1)`, `(200,100)`, `(80,24)`, `(1,0)`, `(0,1)`, `(0,0)`) completes in 1.56s without panics or crashes. Ratatui's widget rendering gracefully clips content on empty/small rects.
   - However, when resizing to dimensions > 255, `SplashApp::set_size` sets `self.terminal_size` to `(w, h)` directly, while `TestBackend` clamps buffer dimensions to `u8::MAX` (255). This creates a state mismatch between the application's tracked terminal size and the rendering backend's area.

2. **Large PTY Output Chunks**:
   - `SplashApp` stores incoming output by appending to `self.raw_output` (`String`).
   - On every frame, `SplashApp::render` executes `self.raw_output.lines().collect::<Vec<&str>>()`, iterating over the full accumulated string.
   - Empirical measurements demonstrate linear slowdown: 50,000 lines (538 KB) renders in 4.1 ms, scaling up to 250,000 lines (2.7 MB) rendering in 18.1 ms. At 60 FPS (16.6 ms per frame), frame rendering drops below target frame rates as output grows unbounded.
   - In `PtyHarness`, calling `String::from_utf8_lossy` on fixed 1024-byte buffer chunks causes multi-byte UTF-8 sequences (such as non-ASCII characters or emojis) that cross the 1024-byte boundary to be split, producing replacement characters (`U+FFFD`, ``) and corrupting the output stream.

3. **Rapid Key Sequences**:
   - Processing 10,000 keypresses takes ~3.5 ms. Leader key toggling (`Ctrl+B Ctrl+B`) and quit key sequences (`Ctrl+B q`) function reliably under rapid input without deadlocks or state desynchronization.
   - In `LeaderState::LeaderPressed`, any key other than `'q'`, `'Q'`, or `Ctrl+B` evaluates to `KeyAction::None` and resets state to `Normal`. The key press (e.g. `'a'`, `'x'`, Arrow keys) is swallowed and lost.
   - `key_event_to_bytes` lacks mappings for terminal navigation keys (Arrow keys, PageUp/Down, Home/End, Delete, Insert, F-keys, Alt key combinations, and `Ctrl+[`), causing those keys to return empty byte vectors.

## 3. Caveats

- Real OS PTY performance under extreme write volume depends on system kernel buffer constraints, which were simulated in `TestHarness` via direct output chunk injection as well as `PtyHarness::spawn` integration tests.
- High terminal dimensions (> 255) were tested against `TestBackend`. Physical terminals may support larger canvas sizes depending on terminal emulator capabilities.

## 4. Conclusion

The core `TestHarness` and `SplashApp` implementations demonstrate high crash resilience under rapid resize operations and high-frequency key inputs (0 panics recorded across all stress tests). However, adversarial empirical challenge identified four notable flaws/limitations:

1. **UTF-8 Stream Corruption in PTY Reader**: `PtyHarness` `from_utf8_lossy` on 1024-byte buffer boundaries corrupts multi-byte UTF-8 characters crossing boundary edges.
2. **Unbounded Render Slowdown**: `SplashApp::render` scans the entire accumulated `raw_output` string on every single frame, causing frame rates to drop below 60 FPS once output exceeds ~200,000 lines.
3. **Leader Mode Key Swallowing**: Unrecognized keys pressed while `LeaderState` is active are dropped (`KeyAction::None`) instead of being handled or forwarded.
4. **Unmapped Terminal Navigation Keys**: Arrow keys, Esc (`Ctrl+[`), F-keys, and Alt modifiers are unmapped in `key_event_to_bytes` and return empty byte vectors.

## 5. Verification Method

To independently verify all findings and benchmark results, execute:
```bash
cargo test --test stress_tests -- --nocapture
```
All 13 empirical stress tests in `tests/stress_tests.rs` verify resize behavior, PTY output performance, key handling edge cases, UTF-8 split boundaries, and leader state machine transitions.
