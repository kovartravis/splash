# Project Plan — Splash Headless Visual Test Harness & Snapshot Validation Suite

## Objective
Implement a headless rendering harness abstraction built on `ratatui::backend::TestBackend` for Splash, with visual buffer inspection helpers/snapshot assertions, and comprehensive integration tests in `tests/` covering UI frame layouts, PTY rendering, leader key states, and layout responsiveness.

## Architecture & Decomposition
- **Milestone 0: Exploration & Codebase Analysis**
  - Explore existing Splash UI rendering structure, terminal backends, main loop event handling, leader key logic, and test suite setup.
  - Query neuron-memory for previous learnings.
  - Define `PROJECT.md` scope document.

- **Milestone 1: Headless Test Harness Abstraction**
  - Build `TestBackend`-based offscreen rendering harness for Splash UI components & main loop views without raw terminal access.
  - Support custom dimensions (80x24, 120x40, etc.).

- **Milestone 2: Visual Buffer & Snapshot Inspection Utilities**
  - Helper assertion macros/methods and formatters for plain-text terminal buffer grid inspection (with borders).
  - Snapshot assertions verifying titles, leader key state (`[LEADER ACTIVE]`), and harness output.

- **Milestone 3: Interactive State & Leader Key Integration Tests**
  - E2E tests in `tests/` simulating key events (`Ctrl+B`, `q`, chars), PTY stream chunks, layout resize events, verifying frame updates.

- **Milestone 4: Verification & Forensic Integrity Audit**
  - Run full test suite via workers (`cargo test`).
  - Perform challenger verification and forensic integrity audit.
