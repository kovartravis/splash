# BRIEFING — 2026-07-16T20:32:19Z

## Mission
Empirically challenge `TestHarness` and `SplashApp` implementations under stress scenarios (rapid resizes, large PTY output chunks, rapid key sequences) and report findings.

## 🔒 My Identity
- Archetype: critic, specialist
- Roles: critic, specialist
- Working directory: /Users/Travis/Repos/splash/.agents/teamwork_preview_challenger_m1_1
- Original parent: 34b0e034-8c14-49c8-a9a9-54f3b0629c10
- Milestone: Milestone 1
- Instance: 1 of 1

## 🔒 Key Constraints
- Review-only — do NOT modify core implementation code unless required for writing tests (tests must be co-located or in designated test directories per layout guidelines; do not put code/tests in .agents/).
- Empirical verification — run tests and code directly, do not rely on unverified claims.

## Current Parent
- Conversation ID: 34b0e034-8c14-49c8-a9a9-54f3b0629c10
- Updated: 2026-07-16T20:32:19Z

## Attack Surface
- **Hypotheses tested**:
  - Rapid resizes (1x1, 200x100, 80x24, 0x0, 500x500): Verified safe rendering, but dimensions >255 reveal a backend clamping discrepancy in `TestBackend` vs `SplashApp.terminal_size`.
  - Large PTY output chunks (10,000 to 250,000 lines): Memory & render time scale linearly without bound. `render()` collects lines across full string on every frame.
  - Multi-byte UTF-8 across 1024-byte PTY buffer reads: Verified `from_utf8_lossy` corrupts split UTF-8 characters into replacement characters (`U+FFFD`).
  - Rapid key sequences & leader toggles: Fast execution with 0 panics.
  - Key mapping & leader state machine: Unrecognized keys in leader state are swallowed (`KeyAction::None`). Arrow keys, F-keys, Alt combinations, and `Ctrl+[` (ESC) are unmapped in `key_event_to_bytes`.
- **Vulnerabilities found**:
  1. UTF-8 multi-byte chunk boundary corruption in `PtyHarness`.
  2. Leader state swallowing unrecognized keys.
  3. Unmapped terminal control/navigation keys (Arrow keys, Esc, Alt).
  4. Linear render slowdown on large `raw_output` (unbounded string accumulation & scan).
  5. Dimension discrepancy for sizes > 255 (backend caps at 255).
- **Untested angles**:
  - Multi-threaded PTY input/output race conditions during continuous stdin/stdout stream collisions.

## Loaded Skills
- None loaded.

## Key Decisions Made
- Authored comprehensive empirical stress test suite in `tests/stress_tests.rs`.
- Recorded findings in neuron memory store.

## Artifact Index
- `/Users/Travis/Repos/splash/.agents/teamwork_preview_challenger_m1_1/ORIGINAL_REQUEST.md`
- `/Users/Travis/Repos/splash/.agents/teamwork_preview_challenger_m1_1/BRIEFING.md`
- `/Users/Travis/Repos/splash/.agents/teamwork_preview_challenger_m1_1/progress.md`
- `/Users/Travis/Repos/splash/tests/stress_tests.rs`
- `/Users/Travis/Repos/splash/.agents/teamwork_preview_challenger_m1_1/handoff.md`
