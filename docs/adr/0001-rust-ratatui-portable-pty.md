# Rust + Ratatui + portable-pty as the technology stack

Splash needs to embed a real terminal emulator (PTY + VT100/ANSI rendering) inside a TUI. Rust with Ratatui and `portable-pty` gives best-in-class PTY handling, predictable performance, and a mature ecosystem for terminal multiplexer-grade work (Zellij is built on the same foundation). Go (BubbleTea) and Node (Ink + node-pty) were considered; both have capable PTY libraries but Rust's ownership model eliminates an entire class of race conditions that arise when a harness process writes to a PTY concurrently with the TUI render loop.
