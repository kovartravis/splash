use std::fs;
use std::sync::Mutex;
use crossterm::event::{KeyCode, KeyModifiers};
use splash::app::HarnessTab;
use splash::pty::HarnessConfig;
use splash::testing::{assert_buffer_contains, TestHarness};

static CWD_MUTEX: Mutex<()> = Mutex::new(());


#[test]
fn test_harness_tab_claude_spawns_mcp_config_guard() {
    let _lock = CWD_MUTEX.lock().unwrap();
    let temp_dir = std::env::temp_dir().join(format!("splash_test_claude_mcp_{}", std::process::id()));

    let _ = fs::remove_dir_all(&temp_dir);
    fs::create_dir_all(&temp_dir).unwrap();

    let claude_config_path = temp_dir.join("claude_desktop_config.json");
    unsafe {
        std::env::set_var("SPLASH_CLAUDE_CONFIG_PATH", &claude_config_path);
    }

    assert!(!claude_config_path.exists());

    {
        let mut tab = HarnessTab::new("claude");
        tab.spawn_pty(24, 80, Some("http://127.0.0.1:9876"));

        assert!(claude_config_path.exists(), "claude_desktop_config.json should be created when claude is spawned");
        let content = fs::read_to_string(&claude_config_path).unwrap();
        let json: serde_json::Value = serde_json::from_str(&content).unwrap();
        assert_eq!(json["mcpServers"]["splash"]["url"], "http://127.0.0.1:9876");
    }

    // After HarnessTab drops, claude_desktop_config.json created by Splash should be cleaned up
    assert!(!claude_config_path.exists(), "claude_desktop_config.json should be deleted on HarnessTab drop");

    unsafe {
        std::env::remove_var("SPLASH_CLAUDE_CONFIG_PATH");
    }
    let _ = fs::remove_dir_all(&temp_dir);
}

#[test]
fn test_e2e_claude_mcp_config_lifecycle() {
    let _lock = CWD_MUTEX.lock().unwrap();
    let temp_dir = std::env::temp_dir().join(format!("splash_e2e_claude_mcp_{}", std::process::id()));

    let _ = fs::remove_dir_all(&temp_dir);
    fs::create_dir_all(&temp_dir).unwrap();

    let claude_config_path = temp_dir.join("claude_desktop_config.json");
    unsafe {
        std::env::set_var("SPLASH_CLAUDE_CONFIG_PATH", &claude_config_path);
    }

    assert!(!claude_config_path.exists());

    let config = HarnessConfig {
        command: "echo".to_string(),
        args: vec!["init".to_string()],
    };
    let mut harness = TestHarness::new(80, 24, config);
    let mcp_url = harness.app.mcp_url.clone().expect("MCP URL missing on app");

    let _ = harness.render_frame();

    // 1. Open Harness Launcher via Ctrl+B h
    harness.press_ctrl('b');
    harness.press_char('h');

    // 2. Type "claude" and press Enter
    for ch in "claude".chars() {
        harness.press_char(ch);
    }
    harness.send_key(KeyCode::Enter, KeyModifiers::empty());

    let buffer = harness.render_frame().clone();
    assert_buffer_contains(&buffer, "[2: claude]");

    // 3. Verify claude_desktop_config.json was created with "splash" server info
    assert!(claude_config_path.exists(), "claude_desktop_config.json should be created after spawning claude via launcher");
    let content = fs::read_to_string(&claude_config_path).unwrap();
    let json: serde_json::Value = serde_json::from_str(&content).unwrap();
    assert_eq!(json["mcpServers"]["splash"]["url"], mcp_url);
    assert_eq!(json["mcpServers"]["splash"]["type"], "sse");

    // 4. Close the active claude tab using Ctrl+B w
    harness.press_ctrl('b');
    harness.press_char('w');

    let _buffer_closed = harness.render_frame().clone();
    assert_eq!(harness.app.tabs.len(), 1);

    // 5. Verify claude_desktop_config.json was cleaned up on tab close
    assert!(!claude_config_path.exists(), "claude_desktop_config.json should be removed after claude tab is closed");

    unsafe {
        std::env::remove_var("SPLASH_CLAUDE_CONFIG_PATH");
    }
    let _ = fs::remove_dir_all(&temp_dir);
}
