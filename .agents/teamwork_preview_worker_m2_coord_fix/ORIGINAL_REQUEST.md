## 2026-07-16T20:44:54Z
You are Worker (teamwork_preview_worker) assigned to fix buffer rect offset indexing in `src/testing/snapshot.rs` for Splash.

Working directory: /Users/Travis/Repos/splash/.agents/teamwork_preview_worker_m2_coord_fix

Identity & Rules:
- MANDATORY INTEGRITY WARNING: DO NOT CHEAT. All implementations must be genuine. DO NOT hardcode test results, create dummy/facade implementations, or circumvent the intended task. A Forensic Auditor will independently verify your work. Integrity violations WILL be detected and your work WILL be rejected.

Task Scope & Detailed Instructions:
1. Examine `src/testing/snapshot.rs` line 88-100 in `format_buffer_grid`.
2. Update cell lookup to use absolute buffer coordinates:
   `let cell = buffer.get(buffer.area.x + x, buffer.area.y + y);`
3. Also defend against right boundary double-width symbol overflow: if `sym_w > 1 && x + (sym_w as u16) > buffer.area.width`, handle bounds cleanly (e.g. advance `x` by 1 or push space) so row length never exceeds `buffer.area.width`.
4. Run `cargo test --all-targets` and `cargo clippy --all-targets -- -D warnings`.
5. Write your completion report to `/Users/Travis/Repos/splash/.agents/teamwork_preview_worker_m2_coord_fix/handoff.md`.
6. Send a completion message back to the parent orchestrator with build & test output details.
