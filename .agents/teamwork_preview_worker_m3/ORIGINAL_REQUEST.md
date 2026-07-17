## 2026-07-16T20:47:03-05:00

You are Worker (teamwork_preview_worker) assigned to implement Milestone 3 (Interactive State & Leader Key Integration Tests) for Splash.

Working directory: /Users/Travis/Repos/splash/.agents/teamwork_preview_worker_m3

Identity & Rules:
- MANDATORY INTEGRITY WARNING: DO NOT CHEAT. All implementations must be genuine. DO NOT hardcode test results, create dummy/facade implementations, or circumvent the intended task. A Forensic Auditor will independently verify your work. Integrity violations WILL be detected and your work WILL be rejected.

Task Scope & Detailed Instructions:
Examine `src/lib.rs`, `src/app.rs`, `src/leader.rs`, `src/pty.rs`, and `src/testing/mod.rs` to understand `TestHarness`, `SplashApp`, `LeaderState`, and key handling logic.

Create two integration test files in `tests/`:
1. `tests/interactive_leader_keys.rs`:
   - Test key event simulation using `TestHarness`:
     - Test pressing `Ctrl+B`: verify `app.leader_state` becomes `LeaderState::Active`, and rendered buffer displays `[LEADER ACTIVE]` in the status bar/footer.
     - Test leader shortcut actions (e.g. pressing `'c'`, `'"'`, `'%''`, `'q'`, or `Esc`). Verify state transition and buffer frame output.
     - Use `assert_snapshot!`, `assert_buffer_contains`, and `assert_buffer_matches_regex` to assert snapshot frame outputs.
2. `tests/pty_integration.rs`:
   - Test PTY stream output injection via `harness.inject_pty_output("Hello, Splash PTY!\nSecond line of output")`.
   - Verify raw output accumulation and rendered text buffer contents.
   - Test terminal layout resize: `harness.resize(100, 30)` and `harness.resize(40, 10)`. Verify app terminal size updates and buffer grid adjusts to new dimensions.
   - Assert visual buffer grid correctness using `assert_snapshot!` and `format_buffer_grid`.

Verification Requirements:
- Run `cargo test --all-targets` and ensure 100% pass across all test targets.
- Run `cargo clippy --all-targets -- -D warnings` and ensure 0 warnings/errors.
- Write your completion report to `/Users/Travis/Repos/splash/.agents/teamwork_preview_worker_m3/handoff.md`.
- Send a completion message back to the parent orchestrator with detailed build & test results.
