use std::time::Duration;
use splash::pty::HarnessConfig;
use splash::testing::TestHarness;
use notify_debouncer_mini::{new_debouncer, notify::RecursiveMode, DebounceEventResult};

#[test]
fn test_auto_reload_file_tabs_integration() {
    let temp_dir = std::env::temp_dir().join("splash_integration_test_cwd");
    std::fs::create_dir_all(&temp_dir).unwrap();
    let original_cwd = std::env::current_dir().unwrap();
    std::env::set_current_dir(&temp_dir).unwrap();

    let config = HarnessConfig {
        command: "bash".to_string(),
        args: vec![],
    };
    let mut harness = TestHarness::new(80, 24, config);

    // Wait briefly for the background thread to attach the watcher to empty cwd
    std::thread::sleep(Duration::from_millis(100));

    let file_path = temp_dir.join("test_auto_reload_integration.txt");
    
    // Initial write
    std::fs::write(&file_path, "Initial Content\n").unwrap();

    // Open file
    harness.app.open_or_focus_file(&file_path).unwrap();
    
    // Trigger tick just in case
    harness.app.tick();
    
    // Assert initial snapshot
    let snapshot1 = harness.buffer_snapshot();
    assert!(snapshot1.contains("Initial Content"));

    // Wait for the background thread to finish setting up the watcher
    std::thread::sleep(Duration::from_millis(200));

    // Write new content
    std::fs::write(&file_path, "Updated Content\nWith more lines\n").unwrap();

    // Wait for debouncer (timeout 50ms, so 200ms is safe)
    std::thread::sleep(Duration::from_millis(200));

    let mut snapshot2 = String::new();
    for _ in 0..20 {
        harness.app.tick();
        snapshot2 = harness.buffer_snapshot();
        if snapshot2.contains("Updated Content") {
            break;
        }
        std::thread::sleep(Duration::from_millis(50));
    }
    
    assert!(snapshot2.contains("Updated Content"));
    
    let _ = std::fs::remove_file(&file_path);
    let _ = std::env::set_current_dir(original_cwd);
}
