use std::fs;
use std::sync::Mutex;
use serde_json::json;
use splash::{setup_panic_hook, McpConfigGuard};

static TEST_MUTEX: Mutex<()> = Mutex::new(());

#[test]
fn test_mcp_config_cleanup_on_panic_created_file() {
    let _lock = TEST_MUTEX.lock().unwrap();
    let temp_dir = std::env::temp_dir().join("splash_test_mcp_panic_1");
    let _ = fs::remove_dir_all(&temp_dir);
    fs::create_dir_all(&temp_dir).unwrap();

    let config_path = temp_dir.join("mcp_config.json");
    setup_panic_hook();

    let path_clone = config_path.clone();
    let handle = std::thread::spawn(move || {
        let guard = McpConfigGuard::register(&path_clone, "splash", "http://127.0.0.1:9999")
            .expect("Failed to register guard");

        assert!(path_clone.exists());

        // Intentionally leak guard to simulate panic without stack unwinding drop
        std::mem::forget(guard);
        panic!("Simulated thread panic");
    });

    let _ = handle.join();

    // Panic hook must trigger McpConfigGuard::cleanup_all() and remove Splash created config file
    assert!(!config_path.exists());

    let _ = fs::remove_dir_all(&temp_dir);
}

#[test]
fn test_mcp_config_cleanup_on_panic_pre_existing_file() {
    let _lock = TEST_MUTEX.lock().unwrap();
    let temp_dir = std::env::temp_dir().join("splash_test_mcp_panic_2");
    let _ = fs::remove_dir_all(&temp_dir);
    fs::create_dir_all(&temp_dir).unwrap();

    let config_path = temp_dir.join("mcp_config.json");
    let initial_json = json!({
        "mcpServers": {
            "existing_server": {
                "url": "http://127.0.0.1:3000",
                "type": "sse"
            }
        }
    });
    fs::write(&config_path, serde_json::to_string_pretty(&initial_json).unwrap()).unwrap();

    setup_panic_hook();

    let path_clone = config_path.clone();
    let handle = std::thread::spawn(move || {
        let guard = McpConfigGuard::register(&path_clone, "splash", "http://127.0.0.1:9999")
            .expect("Failed to register guard");

        assert!(path_clone.exists());

        std::mem::forget(guard);
        panic!("Simulated thread panic with pre-existing config");
    });

    let _ = handle.join();

    // File should exist, but "splash" key removed and "existing_server" preserved
    assert!(config_path.exists());
    let content = fs::read_to_string(&config_path).unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&content).unwrap();
    assert!(parsed["mcpServers"]["splash"].is_null());
    assert_eq!(
        parsed["mcpServers"]["existing_server"]["url"],
        "http://127.0.0.1:3000"
    );

    let _ = fs::remove_dir_all(&temp_dir);
}

#[test]
fn test_mcp_config_cleanup_all_idempotent() {
    let _lock = TEST_MUTEX.lock().unwrap();
    let temp_dir = std::env::temp_dir().join("splash_test_mcp_idempotent");
    let _ = fs::remove_dir_all(&temp_dir);
    fs::create_dir_all(&temp_dir).unwrap();

    let config_path = temp_dir.join("mcp_config.json");
    let guard = McpConfigGuard::register(&config_path, "splash", "http://127.0.0.1:9999")
        .expect("Failed to register guard");

    assert!(config_path.exists());

    // Calling cleanup_all multiple times should be safe
    McpConfigGuard::cleanup_all();
    assert!(!config_path.exists());

    McpConfigGuard::cleanup_all();
    assert!(!config_path.exists());

    // Dropping guard after cleanup_all should be safe
    drop(guard);
    assert!(!config_path.exists());

    let _ = fs::remove_dir_all(&temp_dir);
}
