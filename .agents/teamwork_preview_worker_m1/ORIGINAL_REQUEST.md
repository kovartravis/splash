## 2026-07-16T20:29:57Z

You are Worker 1 (teamwork_preview_worker) implementing Milestone 1: Library Decoupling & Headless Test Harness Abstraction for Splash.

Identity & Working Directory:
- Working directory: /Users/Travis/Repos/splash/.agents/teamwork_preview_worker_m1
- Output report: /Users/Travis/Repos/splash/.agents/teamwork_preview_worker_m1/handoff.md

Task Brief & Scope:
Read the detailed task brief in /Users/Travis/Repos/splash/.agents/orchestrator/m1_task_brief.md and project spec in /Users/Travis/Repos/splash/.agents/orchestrator/PROJECT.md.

Key Tasks:
1. Load neuron memory per SKILL.md by running `neuron learn query "tui ratatui testbackend splash"`.
2. Refactor Splash codebase into a modular library:
   - `src/lib.rs`
   - `src/leader.rs` (`LeaderState`, `KeyAction`, `key_event_to_bytes`)
   - `src/pty.rs` (`HarnessConfig`, `parse_args`, `PtyHarness`)
   - `src/app.rs` (`SplashApp` with generic `render<B: Backend>(&mut self, frame: &mut Frame<B>)`)
   - `src/testing/mod.rs` (`TestHarness` wrapping `Terminal<TestBackend>` and `SplashApp`, supporting custom dimensions 80x24, 120x40, offscreen drawing, key simulation).
   - `src/main.rs` updated to use `splash::...`.
3. Create `tests/headless_harness.rs` testing `TestHarness` with custom dimensions, offscreen rendering, and state updates.
4. Run `cargo test` to verify all tests compile and pass reproducibly.
5. Record action history via `neuron history add` per SKILL.md.

MANDATORY INTEGRITY WARNING:
DO NOT CHEAT. All implementations must be genuine. DO NOT hardcode test results, create dummy/facade implementations, or circumvent the intended task. A Forensic Auditor will independently verify your work. Integrity violations WILL be detected and your work WILL be rejected.

Deliver your handoff report to /Users/Travis/Repos/splash/.agents/teamwork_preview_worker_m1/handoff.md and notify parent upon completion.
