# Splash

Splash is an agent harness shell — a TUI that wraps out-of-the-box agent CLIs (AGY, Claude, Codex, etc.) in a persistent terminal emulator, with a file tree and editor tabs as supporting context. The agent terminal is the hero; everything else serves it.

## Language

**Harness**:
An agent CLI process (e.g. `agy`, `claude`, `codex`) running inside Splash's terminal emulator. The harness is what does the coding.
_Avoid_: agent, bot, assistant, session

**Tab**:
A top-level container in the tab bar representing a specific layout of one or more Panes. Its title dynamically reflects the currently active Pane.
_Avoid_: workspace, window

**Pane**:
A subdivision of a Tab's Main Pane that hosts either a live harness terminal or an editable view of a file.
_Avoid_: split, window, buffer

**Tab Bar**:
The horizontal strip at the top of the screen listing all open tabs.
_Avoid_: tab strip, header

**File Tree**:
The panel on the left side of the screen showing the directory structure rooted at the launch CWD. Fixed — does not follow the active harness's working directory.
_Avoid_: sidebar, explorer, project tree

**Working Tree**:
The directory Splash was launched from. The root anchor for the file tree and the CWD passed to new harnesses.
_Avoid_: project root, workspace

**Main Pane**:
The primary content area of a tab. It can be split horizontally or vertically into multiple sub-panes, allowing harnesses and files to be displayed side-by-side.
_Avoid_: editor area, content area, viewport

**Harness Launcher**:
The input prompt mode (triggered via `Ctrl+B h`) where the user types a harness command (e.g. `agy`, `claude`, `bash`) to spawn a new harness process and open a new Harness Tab.
_Avoid_: launcher prompt, command bar

**Empty Workspace**:
The state when no tabs are open in Splash. Displays a clear message in the Main Pane with guidance to launch a harness or open a file.
_Avoid_: blank screen, null state

**Disk Conflict**:
The state when a file pane has unsaved edits and the underlying file has been modified on disk (typically by the active harness). Splash warns the user rather than silently overwriting either copy.
_Avoid_: merge conflict, save conflict

## Editor phases

The file editor evolves in three phases. The phase boundary is an implementation detail, not a domain concept:

- **Phase 1** — plain text buffer
- **Phase 2** — syntax highlighting
- **Phase 3** — LSP integration (hover, go-to-definition, diagnostics)
