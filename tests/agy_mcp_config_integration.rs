use std::fs;
use std::sync::Mutex;
use crossterm::event::{KeyCode, KeyModifiers};
use splash::app::HarnessTab;
use splash::pty::HarnessConfig;
use splash::testing::{assert_buffer_contains, TestHarness};

static CWD_MUTEX: Mutex<()> = Mutex::new(());

#[test]
fn test_harness_tab_agy_spawns_mcp_config_guard() {
    let _lock = CWD_MUTEX.lock().unwrap();
    let temp_dir = std::env::temp_dir().join(format!("splash_test_agy_mcp_{}", std::process::id()));
    let _ = fs::remove_dir_all(&temp_dir);
    fs::create_dir_all(&temp_dir).unwrap();

    let original_cwd = std::env::current_dir().unwrap();
    std::env::set_current_dir(&temp_dir).unwrap();

    let mcp_config_path = temp_dir.join("mcp_config.json");
    assert!(!mcp_config_path.exists());

    {
        let mut tab = HarnessTab::new("agy");
        tab.spawn_pty(24, 80, Some("http://127.0.0.1:9876"));

        assert!(mcp_config_path.exists(), "mcp_config.json should be created when agy is spawned");
        let content = fs::read_to_string(&mcp_config_path).unwrap();
        let json: serde_json::Value = serde_json::from_str(&content).unwrap();
        assert_eq!(json["mcpServers"]["splash"]["url"], "http://127.0.0.1:9876");
    }

    // After HarnessTab drops, mcp_config.json should be cleaned up
    assert!(!mcp_config_path.exists(), "mcp_config.json should be deleted on HarnessTab drop");

    let _ = std::env::set_current_dir(original_cwd);
    let _ = fs::remove_dir_all(&temp_dir);
}

#[test]
fn test_e2e_agy_mcp_config_lifecycle() {
    let _lock = CWD_MUTEX.lock().unwrap();
    let temp_dir = std::env::temp_dir().join(format!("splash_e2e_mcp_{}", std::process::id()));
    let _ = fs::remove_dir_all(&temp_dir);
    fs::create_dir_all(&temp_dir).unwrap();

    let original_cwd = std::env::current_dir().unwrap();
    std::env::set_current_dir(&temp_dir).unwrap();

    let mcp_config_path = temp_dir.join("mcp_config.json");
    assert!(!mcp_config_path.exists());

    let config = HarnessConfig {
        command: "echo".to_string(),
        args: vec!["init".to_string()],
    };
    let mut harness = TestHarness::new(80, 24, config);

    let mcp_url = harness.app.mcp_url.clone().expect("MCP URL missing on app");

    // Render initial frame
    let _ = harness.render_frame();

    // 1. Open Harness Launcher via Ctrl+B h
    harness.press_ctrl('b');
    harness.press_char('h');

    // 2. Type "agy" and press Enter
    harness.press_char('a');
    harness.press_char('g');
    harness.press_char('y');
    harness.send_key(KeyCode::Enter, KeyModifiers::empty());

    // Render frame after spawning agy tab
    let buffer = harness.render_frame().clone();
    assert_buffer_contains(&buffer, "[2: agy]");

    // 3. Verify mcp_config.json was created and populated with "splash" server info
    assert!(mcp_config_path.exists(), "mcp_config.json should be created after spawning agy via launcher");
    let content = fs::read_to_string(&mcp_config_path).unwrap();
    let json: serde_json::Value = serde_json::from_str(&content).unwrap();
    assert_eq!(json["mcpServers"]["splash"]["url"], mcp_url);
    assert_eq!(json["mcpServers"]["splash"]["type"], "sse");

    // 4. Close the active agy tab using Ctrl+B w
    harness.press_ctrl('b');
    harness.press_char('w');

    let _buffer_closed = harness.render_frame().clone();
    assert_eq!(harness.app.tabs.len(), 1);

    // 5. Verify mcp_config.json was cleaned up on tab close
    assert!(!mcp_config_path.exists(), "mcp_config.json should be removed after agy tab is closed");

    let _ = std::env::set_current_dir(original_cwd);
    let _ = fs::remove_dir_all(&temp_dir);
}
