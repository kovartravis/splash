# Splash

Splash is an agent harness shell — a TUI that wraps out-of-the-box agent CLIs (AGY, Claude, Codex, etc.) in a persistent terminal emulator, with a file tree and editor tabs as supporting context. The agent terminal is the hero; everything else serves it.

## Language

**Harness**:
An agent CLI process (e.g. `agy`, `claude`, `codex`) running inside Splash's terminal emulator. The harness is what does the coding.
_Avoid_: agent, bot, assistant, session

**Harness Tab**:
A tab in the tab bar that hosts a live harness terminal. Distinguished from file tabs by a color-coded border (phase 1) and a working/idle status indicator (phase 2).
_Avoid_: agent tab, terminal tab, session tab

**File Tab**:
A tab in the tab bar that hosts an editable view of a file from the working tree. Saves explicitly on `Ctrl+S`; auto-refreshes when the file changes on disk.
_Avoid_: editor tab, buffer

**Tab Bar**:
The horizontal strip at the top of the screen listing all open harness tabs and file tabs.
_Avoid_: tab strip, header

**File Tree**:
The panel on the left side of the screen showing the directory structure rooted at the launch CWD. Fixed — does not follow the active harness's working directory.
_Avoid_: sidebar, explorer, project tree

**Working Tree**:
The directory Splash was launched from. The root anchor for the file tree and the CWD passed to new harnesses.
_Avoid_: project root, workspace

**Main Pane**:
The primary content area that shows either the active harness terminal or the active file tab. One view at a time — no splits.
_Avoid_: editor area, content area, viewport

**Command Palette**:
The picker that appears when opening a new harness tab. Lists harnesses detected on `$PATH` and spawns the selected one in a new harness tab.
_Avoid_: launcher, menu

**Disk Conflict**:
The state when a file tab has unsaved edits and the underlying file has been modified on disk (typically by the active harness). Splash warns the user rather than silently overwriting either copy.
_Avoid_: merge conflict, save conflict

## Editor phases

The file tab editor evolves in three phases. The phase boundary is an implementation detail, not a domain concept:

- **Phase 1** — plain text buffer
- **Phase 2** — syntax highlighting
- **Phase 3** — LSP integration (hover, go-to-definition, diagnostics)
