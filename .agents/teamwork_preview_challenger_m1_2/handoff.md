# Empirical Challenge Handoff Report — Challenger 2 (Milestone 1)

## 1. Observation

Empirical testing of `LeaderState`, `TestHarness`, and `key_event_to_bytes` in `src/leader.rs`, `src/app.rs`, `src/testing/mod.rs`, and `tests/empirical_challenge_m1_2.rs` yielded the following findings:

1. **`LeaderState` handling of `Ctrl+B` followed by non-q keys**:
   - File: `src/leader.rs:37-46`
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
   - Observed behavior: When in `LeaderPressed` state, typing any non-q key (e.g., `'x'`, `b` without CONTROL, `Esc`, `Enter`, or arrow keys) resets `LeaderState` to `Normal`, but returns `KeyAction::None`. The pressed key is swallowed and never forwarded to the PTY.
   - Quitting behavior: `KeyCode::Char('q')` and `KeyCode::Char('Q')` both trigger `KeyAction::Quit`. In addition, `Ctrl+Q` or `Shift+Q` also triggers `KeyAction::Quit` because `key.code` matches `KeyCode::Char('q')` / `KeyCode::Char('Q')`.
   - Literal `Ctrl+B` passthrough: Sending `Ctrl+B` followed by `Ctrl+B` returns `KeyAction::Forward(vec![0x02])` and resets state to `Normal`.

2. **Unmapped Non-Alphabetic Ctrl Chords**:
   - File: `src/leader.rs:52-63`
   ```rust
   if key.modifiers.contains(KeyModifiers::CONTROL) {
       match key.code {
           KeyCode::Char(c) => {
               let lower = c.to_ascii_lowercase();
               if lower.is_ascii_lowercase() {
                   vec![(lower as u8) - b'a' + 1]
               } else {
                   vec![]
               }
           }
           _ => vec![],
       }
   }
   ```
   - Observed behavior: `key_event_to_bytes` converts `Ctrl+a` through `Ctrl+z` to bytes `0x01`–`0x1A`. However, `lower.is_ascii_lowercase()` returns `false` for non-alphabetic characters.
   - Consequently, `Ctrl+[` (Escape, 0x1B), `Ctrl+]` (0x1D), `Ctrl+\` (0x1C), `Ctrl+^` (0x1E), `Ctrl+_` (0x1F), `Ctrl+@` (0x00), `Ctrl+Enter`, `Ctrl+Backspace`, and `Ctrl+Tab` return `vec![]` (`KeyAction::None`). They are silently dropped.

3. **Unmapped Arrow Keys, Navigation Keys, and Function Keys**:
   - File: `src/leader.rs:64-76`
   ```rust
   match key.code {
       KeyCode::Char(c) => { ... }
       KeyCode::Enter => vec![b'\r'],
       KeyCode::Backspace => vec![0x7f],
       KeyCode::Tab => vec![b'\t'],
       KeyCode::Esc => vec![0x1b],
       _ => vec![],
   }
   ```
   - Observed behavior: `KeyCode::Up`, `KeyCode::Down`, `KeyCode::Left`, `KeyCode::Right`, `KeyCode::Home`, `KeyCode::End`, `KeyCode::PageUp`, `KeyCode::PageDown`, `KeyCode::Delete`, `KeyCode::Insert`, `KeyCode::F(1)`..`KeyCode::F(12)`, and `KeyCode::BackTab` return `vec![]` -> `KeyAction::None`.
   - Result: All directional arrow navigation, history scrolling in shell, text editing navigation, and F-key commands are ignored and swallowed by Splash.

4. **Multi-byte Unicode Processing**:
   - File: `src/leader.rs:66-68`
   ```rust
   KeyCode::Char(c) => {
       let mut buf = [0; 4];
       c.encode_utf8(&mut buf).as_bytes().to_vec()
   }
   ```
   - Observed behavior: In `Normal` state (without CONTROL modifier), multi-byte UTF-8 characters encode correctly:
     - 2-byte `'é'` -> `vec![0xC3, 0xA9]`
     - 3-byte `'€'` -> `vec![0xE2, 0x82, 0xAC]`
     - 4-byte `'🦀'` -> `vec![0xF0, 0x9F, 0xA6, 0x80]`
   - Multi-byte characters with CONTROL modifier or in `LeaderPressed` state return `KeyAction::None`.

5. **`TestHarness` Visual & State Transition Fidelity**:
   - File: `src/testing/mod.rs:26-57`, `src/app.rs:36-48`
   - Observed behavior: `TestHarness` accurately reflects `LeaderState` transitions. When `LeaderPressed` is active, `render_frame()` outputs `[LEADER ACTIVE]` in the title block of the terminal widget. When transitioning back to `Normal`, `[LEADER ACTIVE]` disappears.

---

## 2. Logic Chain

1. From **Observation 1**, `LeaderState::handle_key` sets `*self = LeaderState::Normal` on any non-q input when in `LeaderPressed` state, but defaults to `KeyAction::None`. Therefore, typing `Ctrl+B` followed by any key other than `q`/`Q` or `Ctrl+B` (e.g., `Ctrl+B x`) results in `x` being lost rather than forwarded to the PTY.
2. From **Observation 2**, `key_event_to_bytes` checks `if lower.is_ascii_lowercase()` when handling `KeyModifiers::CONTROL`. Non-alphabetic characters (such as `[` or `]`) fail this guard and return `vec![]`. Therefore, common terminal shortcuts like `Ctrl+[` (Vim mode Escape) cannot be sent to the harness.
3. From **Observation 3**, `key_event_to_bytes` explicitly lists only `KeyCode::Char`, `KeyCode::Enter`, `KeyCode::Backspace`, `KeyCode::Tab`, and `KeyCode::Esc` in its `else` branch. Unhandled enum variants (arrows, navigation, F-keys) hit `_ => vec![]`. Therefore, shell navigation (e.g. up arrow for history) is non-functional in the harness terminal.
4. From **Observation 4**, `c.encode_utf8` generates standard UTF-8 byte sequences for Unicode code points up to 4 bytes in `Normal` mode. However, in `LeaderPressed` mode or with `CONTROL`, Unicode characters are swallowed due to matching `_ => KeyAction::None` or `is_ascii_lowercase() == false`.
5. From **Observation 5**, `TestHarness` wraps `SplashApp` and correctly exercises `handle_key_event` and `render`, providing full fidelity for headless testing of key actions and rendered buffer states.

---

## 3. Caveats

- Hardware terminal resize signaling (SIGWINCH under rapid resize pressure during key handling) was not stress-tested at the OS kernel level in headless mode.
- PTY terminal mouse reporting events (e.g. mouse click/scroll crossterm events) were not evaluated as `key_event_to_bytes` only inspects `KeyEvent`.

---

## 4. Conclusion

- **Verdict**: `LeaderState` and `TestHarness` fundamental state transitions work cleanly for valid basic sequences (`Ctrl+B` -> `q` -> `Quit`, `Ctrl+B` -> `Ctrl+B` -> literal `Ctrl+B`, multi-byte UTF-8 string inputs in `Normal` mode).
- **Key Flaws / Failure Modes Identified**:
  1. **Non-q key swallowing**: Any non-q key following `Ctrl+B` is swallowed (`KeyAction::None`).
  2. **Missing Arrow/Navigation key support**: Arrow keys, Home, End, Delete, PageUp/PageDown, F-keys return `vec![]` and cannot be sent to PTY processes.
  3. **Missing non-alphabetic Ctrl-chords**: `Ctrl+[` (Escape), `Ctrl+]`, `Ctrl+\`, `Ctrl+Enter`, `Ctrl+Backspace` are swallowed.

---

## 5. Verification Method

To independently reproduce and verify all empirical findings:

1. Run the test suite containing empirical challenge test cases:
   ```bash
   cargo test --test empirical_challenge_m1_2
   ```
   All 7 test cases in `tests/empirical_challenge_m1_2.rs` pass and demonstrate the exact behavior described above.

2. Run the complete workspace test suite:
   ```bash
   cargo test
   ```

---

## Adversarial Challenge Report

### Challenge Summary
- **Overall risk assessment**: MEDIUM
- **Primary concerns**: Missing key mappings for Arrow keys, Navigation keys, and non-alphabetic Ctrl chords will impact usability when running interactive CLI tools (like Vim, bash history, line editors) inside Splash terminal harnesses.

### Challenges

#### 1. [Medium] Arrow Keys & Control Chords Swallowed by `key_event_to_bytes`
- **Assumption challenged**: `key_event_to_bytes` only needs basic character and enter/backspace/tab/esc support.
- **Attack scenario**: User opens an interactive bash session in Splash and presses Up Arrow to view command history or Left/Right arrows to edit line.
- **Blast radius**: Arrow keys return `vec![]` (`KeyAction::None`), leaving shell history and cursor navigation broken.
- **Mitigation**: Expand `key_event_to_bytes` to translate `KeyCode::Up` (`\x1b[A`), `KeyCode::Down` (`\x1b[B`), `KeyCode::Right` (`\x1b[C`), `KeyCode::Left` (`\x1b[D`), and non-alphabetic Ctrl chords (`Ctrl+[` -> `\x1b`).

#### 2. [Low] Non-q Keys Swallowed in Leader Mode
- **Assumption challenged**: User will only type `q` or `Ctrl+B` after activating leader key.
- **Attack scenario**: User accidentally presses `Ctrl+B` followed by another key (e.g., `Ctrl+B` then `x` or `Ctrl+B` then `b` without Ctrl).
- **Blast radius**: The subsequent key is dropped (`KeyAction::None`).
- **Mitigation**: Decide whether Leader mode should pass through the unhandled key (or cancel leader mode without dropping the key).

### Stress Test Results

| Scenario | Expected Behavior | Actual Behavior | Pass/Fail |
|---|---|---|---|
| `Ctrl+B` -> `q` | Return `Quit`, state -> `Normal` | Returns `Quit`, state -> `Normal` | PASS |
| `Ctrl+B` -> `Ctrl+B` | Return `Forward([0x02])`, state -> `Normal` | Returns `Forward([0x02])`, state -> `Normal` | PASS |
| `Ctrl+B` -> `'x'` | Reset state -> `Normal` | Resets state -> `Normal`, returns `None` (key swallowed) | PASS (swallow verified) |
| Multi-byte UTF-8 (`'🦀'`) | Return `Forward([0xF0, 0x9F, 0xA6, 0x80])` | Returns `Forward([0xF0, 0x9F, 0xA6, 0x80])` | PASS |
| `Ctrl+[` | Return `Forward([0x1B])` | Returns `KeyAction::None` (swallowed) | FAIL (Gap in input mapping) |
| `KeyCode::Up` | Return `Forward("\x1b[A")` | Returns `KeyAction::None` (swallowed) | FAIL (Gap in input mapping) |
| `TestHarness` snapshot | Title contains `[LEADER ACTIVE]` when leader is active | Title contains `[LEADER ACTIVE]` when leader is active | PASS |

### Unchallenged Areas
- Signal delivery for window resize (`SIGWINCH`) during high-frequency terminal rendering.
