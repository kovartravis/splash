use std::fs;
use splash::app::HarnessTab;

#[test]
fn test_harness_tab_agy_spawns_mcp_config_guard() {
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
