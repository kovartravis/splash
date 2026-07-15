# Splash POC Spec

## Goal

Prove that a TUI can host a live agent harness terminal alongside a navigable file tree and read-only file viewer, end-to-end, in a single Rust binary.

**Success criterion:** Run `splash agy` in a project directory. Type a prompt. The harness edits a file. Navigate to it in the file tree, open it, and see the updated contents — without leaving Splash.

---

## Layout

```
┌─────────────────────────────────────────────────────────┐
│  [1: agy]  [2: main.rs]  [3: README.md]                 │  ← Tab Bar
├──────────┬──────────────────────────────────────────────┤
│ src/     │                                              │
│ ├ main   │        Main Pane                             │
│ │  .rs   │   (harness terminal or file viewer)          │
│ └ lib    │                                              │
│   .rs    │                                              │
│ README   │                                              │
│ .md      │                                              │
│ Cargo    │                                              │
│ .toml    │                                              │
└──────────┴──────────────────────────────────────────────┘
   File Tree         (no status bar for POC)
```

Proportions: file tree ~20% width, main pane ~80%. Fixed — not resizable in the POC.

---

## Features

### 1. CLI invocation

```
splash <harness-command>
```

- `<harness-command>` is a single token (e.g. `agy`, `claude`, `codex`). It must be on `$PATH` — Splash does not validate it; it lets the PTY fail naturally if the command isn't found.
- The working tree is `$PWD` at invocation.
- Splash exits with a non-zero code if the initial harness process fails to start.

### 2. Tab bar

- Displayed at the top of the screen, full width.
- Tabs are numbered from `1`. Harness tabs are visually distinct: colour-coded left border (the specific colour is an implementation choice for the POC).
- Active tab is highlighted. Inactive tabs are dimmed.
- No tab scrolling for the POC — if tabs overflow the width, they truncate.

### 3. File tree

- Rooted at the working tree. Does not follow the harness's CWD.
- Displays directories and files. Hidden files (dotfiles) are shown.
- Directories are collapsible. Default state: top-level entries expanded, subdirectories collapsed.
- Keyboard navigation:
  - `↑` / `↓` — move cursor up/down
  - `→` or `Enter` on a directory — expand it
  - `←` or `Enter` on an expanded directory — collapse it
  - `Enter` on a file — open it in a new file tab (or focus its existing tab if already open)
- No file tree refresh for the POC — the tree reflects the working tree at launch. New files created by the harness are not visible until Splash restarts.

### 4. Harness tab (Main Pane — harness terminal)

- A full PTY terminal emulator backed by `portable-pty`.
- ANSI/VT100 rendering — colour, bold, cursor movement all work.
- When the harness tab is active and the main pane is focused, all keystrokes (except the leader prefix `Ctrl+B`) are forwarded to the PTY.
- The harness tab cannot be closed in the POC. Quitting the harness process (e.g. typing `exit`) exits Splash.

### 5. File tab (Main Pane — file viewer)

- Read-only. Displays raw file bytes as UTF-8 text. Non-UTF-8 bytes are replaced with `<?>`.
- Scrollable: `↑`/`↓` scroll line by line; `PgUp`/`PgDn` scroll by half-screen.
- **Auto-refresh**: Splash watches the file with `inotify` (Linux) / `kqueue` (macOS). When a change is detected, the tab reloads automatically. No conflict warning is needed in the POC (file is read-only).
- File tabs can be closed with `Ctrl+B w`.

### 6. Keybindings

All navigation uses a `Ctrl+B` leader prefix so that harness input is never accidentally captured.

| Chord | Action |
|---|---|
| `Ctrl+B` `1`…`9` | Switch to tab by number |
| `Ctrl+B` `←` | Focus file tree |
| `Ctrl+B` `→` | Focus main pane |
| `Ctrl+B` `w` | Close the active file tab (no-op on a harness tab) |
| `Ctrl+B` `q` | Quit Splash (kills all harness processes) |

When the file tree is focused, arrow keys navigate it. When the main pane is focused and showing a file tab, arrow keys scroll the file. When the main pane is focused and showing a harness tab, all input goes to the PTY.

---

## Out of scope

The following are explicitly deferred to post-POC iterations:

- Command palette / multiple harness tabs
- Editable file tabs (`Ctrl+S`, text buffer, cursor)
- Disk conflict detection
- Mouse support
- File tree live refresh (new files appearing while Splash runs)
- Resizable panes
- Syntax highlighting / LSP
- Detachable sessions (`--attach`)
- Status bar
- Tab overflow scrolling

---

## Acceptance test (manual)

1. `cd` into any project with at least one file.
2. Run `splash agy` (or any available harness).
3. Confirm the file tree and harness terminal are visible.
4. Type a prompt asking the harness to append a line to an existing file.
5. While the harness works, press `Ctrl+B ←` to focus the file tree.
6. Navigate to the modified file with arrow keys. Press `Enter` to open it.
7. Press `Ctrl+B 2` to switch to the file tab.
8. Confirm the file contents reflect the harness's edit (auto-refresh).
9. Press `Ctrl+B q` to quit. Confirm clean exit.
