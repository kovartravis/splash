# BRIEFING — 2026-07-16T20:32:10Z

## Mission
Empirically challenge LeaderState and TestHarness input state transitions and edge cases in Splash.

## 🔒 My Identity
- Archetype: Empirical Challenger
- Roles: critic, specialist
- Working directory: /Users/Travis/Repos/splash/.agents/teamwork_preview_challenger_m1_2
- Original parent: 34b0e034-8c14-49c8-a9a9-54f3b0629c10
- Milestone: Milestone 1
- Instance: Challenger 2

## 🔒 Key Constraints
- Review-only — do NOT modify implementation code.
- Empirically reproduce all bugs — if cannot reproduce empirically, it does not count.

## Current Parent
- Conversation ID: 34b0e034-8c14-49c8-a9a9-54f3b0629c10
- Updated: 2026-07-16T20:32:10Z

## Review Scope
- **Files to review**: `LeaderState`, `TestHarness`, `key_event_to_bytes`, input handling code in splash codebase
- **Interface contracts**: PROJECT.md / SCOPE.md / ADR 0004
- **Review criteria**: Correctness of input state transitions, edge case handling (`Ctrl+B` + non-q keys, escape sequences, multi-byte unicode input, ctrl-chords).

## Key Decisions Made
- Executed `neuron learn query` and logged initialization.
- Added comprehensive empirical test suite `tests/empirical_challenge_m1_2.rs`.
- Empirically verified `LeaderState` state machine, `TestHarness` rendering, UTF-8 multi-byte handling, non-q swallowing, non-alpha ctrl-chords, and arrow/navigation key omissions.

## Artifact Index
- `/Users/Travis/Repos/splash/.agents/teamwork_preview_challenger_m1_2/ORIGINAL_REQUEST.md` — Original request log
- `/Users/Travis/Repos/splash/.agents/teamwork_preview_challenger_m1_2/BRIEFING.md` — Agent working memory
- `/Users/Travis/Repos/splash/tests/empirical_challenge_m1_2.rs` — Empirical test harness suite
- `/Users/Travis/Repos/splash/.agents/teamwork_preview_challenger_m1_2/handoff.md` — Final handoff report

## Attack Surface
- **Hypotheses tested**:
  1. Non-q keys following `Ctrl+B` are swallowed. (CONFIRMED)
  2. Non-alphabetic Ctrl-chords (`Ctrl+[`, `Ctrl+]`, `Ctrl+\`) return empty byte vectors. (CONFIRMED)
  3. Arrow keys, F-keys, Home/End/PgUp/PgDn/Del are unmapped and swallowed. (CONFIRMED)
  4. Multi-byte UTF-8 chars work in Normal mode but swallowed in Leader/Ctrl modes. (CONFIRMED)
  5. TestHarness correctly reflects `[LEADER ACTIVE]` rendering in title block. (CONFIRMED)
- **Vulnerabilities found**:
  - `key_event_to_bytes` lacks mapping for Arrow keys, F-keys, Navigation keys (Home/End/Delete/etc.).
  - `key_event_to_bytes` drops non-alphabetic Ctrl chords (`Ctrl+[`, `Ctrl+]`, `Ctrl+\`, `Ctrl+Enter`, etc.).
  - `LeaderState` swallows non-q keys typed after `Ctrl+B` without passing them through to PTY.
- **Untested angles**: Hardware terminal resize signals (SIGWINCH under high pressure).

## Loaded Skills
- `neuron-memory`: Persistent context and learnings management.
