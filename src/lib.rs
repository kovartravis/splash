pub mod app;
pub mod leader;
pub mod mcp_proxy;
pub mod mcp_guard;
pub mod pty;
pub mod testing;
pub mod tree;
pub use app::{HarnessTab, SplashApp, Tab};
pub use leader::{key_event_to_bytes, KeyAction, LeaderState};
pub use mcp_guard::{install_signal_and_panic_hooks, setup_panic_hook, setup_signal_handlers, McpConfigGuard};
pub use pty::{parse_args, HarnessConfig, PtyHarness};
pub use tree::{FileNode, FileTree};


