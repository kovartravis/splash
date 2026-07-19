# Context Summary — Splash Project

## System Purpose
Splash is an agent harness shell — a TUI wrapping agent CLIs (AGY, Claude, Codex, etc.) in a persistent terminal emulator with file tree and editor tabs.

## Current Goal
Add a headless visual test harness and snapshot validation suite using `ratatui::backend::TestBackend` to automatically verify TUI frame layouts, PTY output rendering, leader key states, and layout responsiveness.

## Key Files & Requirements
- ORIGINAL_REQUEST.md: /Users/Travis/Repos/splash/.agents/ORIGINAL_REQUEST.md
- Requirements: R1 (Headless Harness), R2 (Buffer & Snapshot Inspection), R3 (Interactive State & Leader Key Tests).
- Constraints: TDD, neuron-memory skill compliance, zero-cheating integrity checks, non-interactive CI compatibility.
