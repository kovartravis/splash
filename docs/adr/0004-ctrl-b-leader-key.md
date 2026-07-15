# Ctrl+B leader-key model for all TUI navigation

Splash uses a `Ctrl+B` leader prefix for all navigation and control chords (tab switching, pane focus, quit). This is borrowed directly from tmux's convention. The alternative — modifier-key shortcuts like `Alt+←` without a leader — risks collision with harness CLIs that consume `Alt` sequences (many readline-based CLIs do). `Ctrl+B` is almost universally unused by agent harnesses, making it a safe, unambiguous escape hatch that guarantees raw PTY input is never accidentally captured by Splash's keybinding layer.

## Considered Options

- **Alt+arrows / Ctrl+arrows (no leader)** — simpler to implement and discover, but `Alt` sequences are consumed by readline and many TUI programs that a harness might run as a subprocess.
- **Ctrl+B leader (chosen)** — one extra keypress per action, but zero collision risk with any harness input.
