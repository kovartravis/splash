# 0005: Harness Tab Closure & Harness Launcher

## Context

Originally, the Splash POC specified that Harness Tab 1 could not be closed and that closing the harness process was the only way to exit Splash. As Splash evolves to support opening multiple files and running flexible agent harnesses:

1. Users need to close Harness Tabs when they are finished with a harness run.
2. Closing a Harness Tab must cleanly terminate its underlying PTY process to prevent orphaned background processes.
3. Closing all open tabs transitions Splash into an **Empty Workspace** state rather than abruptly terminating the application.
4. Users can launch new harnesses at any time via a dedicated `Ctrl+B h` **Harness Launcher** prompt.

## Decision

1. `Ctrl+B w` can close both File Tabs and Harness Tabs.
2. Closing a Harness Tab kills its underlying PTY process (`PtyHarness`).
3. When `tabs.is_empty()`, Splash displays an **Empty Workspace** screen in the Main Pane with instructions to open a file or launch a new harness.
4. `Ctrl+B h` opens a **Harness Launcher** text prompt to type any harness command on `$PATH`, spawning a new Harness Tab on `Enter`.

## Consequences

- Users have complete control over tab lifecycle and process cleanup.
- Splash remains persistent even when 0 tabs are open.
- Key navigation (`Ctrl+B`) remains active at all times.
