
use splash::pty::HarnessConfig;
use splash::testing::TestHarness;
use splash::tree::FileTree;
use serde_json::Value;

#[test]
fn test_mcp_server_and_list_layout() {
    let temp_dir = std::env::temp_dir().join(format!("splash_mcp_{}", std::process::id()));
    let _ = std::fs::create_dir_all(&temp_dir);
    let empty_tree = FileTree::new(&temp_dir).unwrap();
    
    // Command that prints the environment variable
    let config = HarnessConfig {
        command: "sh".to_string(),
        args: vec!["-c".to_string(), "echo MCP_URL=$SPLASH_MCP_URL".to_string()],
    };
    
    let mut harness = TestHarness::with_file_tree(80, 24, config, empty_tree);
    
    let mcp_url = harness.app.mcp_url.clone().expect("SPLASH_MCP_URL was not found on app");
    assert!(mcp_url.starts_with("http://127.0.0.1:"));
    
    let request_body = serde_json::json!({
        "jsonrpc": "2.0",
        "method": "tools/call",
        "params": {
            "name": "list_layout",
            "arguments": {}
        },
        "id": 1
    });
    
    use std::io::{Read, Write};
    use std::net::TcpStream;
    let addr = mcp_url.replace("http://", "");
    let mut stream = TcpStream::connect(&addr).expect("Failed to connect to MCP server");
    
    let req_body_str = serde_json::to_string(&request_body).unwrap();
    let request_str = format!(
        "POST /mcp HTTP/1.1\r\nHost: {}\r\nContent-Length: {}\r\nContent-Type: application/json\r\n\r\n{}",
        addr,
        req_body_str.len(),
        req_body_str
    );
    stream.write_all(request_str.as_bytes()).unwrap();
    
    // Pump app event loop to process the request
    for _ in 0..50 {
        harness.app.tick();
        std::thread::sleep(std::time::Duration::from_millis(10));
    }
    
    // Set read timeout
    stream.set_read_timeout(Some(std::time::Duration::from_secs(1))).unwrap();
    
    let mut response = String::new();
    let _ = stream.read_to_string(&mut response); // May return Error(WouldBlock) when socket is closed or timeout, but we just want whatever was read
    
    let body = response.split("\r\n\r\n").nth(1).expect("Invalid HTTP response format");
    let res_json: Value = serde_json::from_str(body).expect("Response is not JSON");
    
    let result = &res_json["result"];
    assert!(!result.is_null(), "Expected a result in the JSON-RPC response");
    
    let content = result["content"].as_array().expect("Result should have content array");
    let text_content = content[0]["text"].as_str().expect("Content should have text");
    
    let parsed_layout: Value = serde_json::from_str(text_content).expect("Text content should be JSON layout");
    assert_eq!(parsed_layout["tabs"].as_array().unwrap().len(), 1);
    
    let first_tab = &parsed_layout["tabs"][0];
    assert_eq!(first_tab["active_pane_id"].as_u64().unwrap(), 0);
}

#[test]
fn test_mcp_server_open_file() {
    let temp_dir = std::env::temp_dir().join(format!("splash_mcp_open_{}", std::process::id()));
    let _ = std::fs::create_dir_all(&temp_dir);
    // Create a dummy file to open
    std::fs::write(temp_dir.join("test_file.rs"), "fn test() {}").unwrap();
    let empty_tree = FileTree::new(&temp_dir).unwrap();
    
    let config = HarnessConfig {
        command: "sh".to_string(),
        args: vec!["-c".to_string(), "echo MCP_URL=$SPLASH_MCP_URL".to_string()],
    };
    
    let mut harness = TestHarness::with_file_tree(80, 24, config, empty_tree);
    let mcp_url = harness.app.mcp_url.clone().expect("SPLASH_MCP_URL was not found on app");
    
    let request_body = serde_json::json!({
        "jsonrpc": "2.0",
        "method": "tools/call",
        "params": {
            "name": "open_file",
            "arguments": {
                "location": "split_right",
                "file_path": temp_dir.join("test_file.rs").to_str().unwrap()
            }
        },
        "id": 2
    });
    
    use std::io::{Read, Write};
    use std::net::TcpStream;
    let addr = mcp_url.replace("http://", "");
    let mut stream = TcpStream::connect(&addr).expect("Failed to connect to MCP server");
    
    let req_body_str = serde_json::to_string(&request_body).unwrap();
    let request_str = format!(
        "POST /mcp HTTP/1.1\r\nHost: {}\r\nContent-Length: {}\r\nContent-Type: application/json\r\n\r\n{}",
        addr,
        req_body_str.len(),
        req_body_str
    );
    stream.write_all(request_str.as_bytes()).unwrap();
    
    for _ in 0..50 {
        harness.app.tick();
        std::thread::sleep(std::time::Duration::from_millis(10));
    }
    
    // Assert visual split
    assert_eq!(harness.app.tabs.len(), 1);
    assert_eq!(harness.app.tabs[0].panes().len(), 2);
    
    // Test that the active focus shifted to the new file pane
    // The active pane should be the new file
    let active_pane_id = harness.app.tabs[0].active_pane_id;
    let active_pane = harness.app.tabs[0].panes().into_iter().find(|p| p.id == active_pane_id).unwrap();
    if let splash::app::PaneContent::File(f) = &active_pane.content {
        assert!(f.path.ends_with("test_file.rs"));
    } else {
        panic!("Active pane is not a file");
    }
}

#[test]
fn test_mcp_server_close_pane() {
    let temp_dir = std::env::temp_dir().join(format!("splash_mcp_close_{}", std::process::id()));
    let _ = std::fs::create_dir_all(&temp_dir);
    let empty_tree = FileTree::new(&temp_dir).unwrap();
    
    let config = HarnessConfig {
        command: "sh".to_string(),
        args: vec![],
    };
    
    let mut harness = TestHarness::with_file_tree(80, 24, config, empty_tree);
    
    // Split pane to have 2 panes
    harness.app.split_active_pane(splash::app::SplitDirection::Vertical, splash::app::PaneContent::File(splash::app::FileTab::open(temp_dir.join("dummy")).unwrap_or_else(|_| splash::app::FileTab { path: std::path::PathBuf::from("dummy"), content: String::new(), scroll_offset: 0 })));
    assert_eq!(harness.app.tabs[0].panes().len(), 2);
    let pane_id_to_close = harness.app.tabs[0].active_pane_id;
    let initial_pane_id = harness.app.tabs[0].panes().iter().find(|p| p.id != pane_id_to_close).unwrap().id;
    
    let mcp_url = harness.app.mcp_url.clone().unwrap();
    let request_body = serde_json::json!({
        "jsonrpc": "2.0",
        "method": "tools/call",
        "params": {
            "name": "close_pane",
            "arguments": {
                "pane_id": pane_id_to_close
            }
        },
        "id": 3
    });
    
    let addr = mcp_url.replace("http://", "");
    let mut stream = std::net::TcpStream::connect(&addr).unwrap();
    let req_body_str = serde_json::to_string(&request_body).unwrap();
    let request_str = format!(
        "POST /mcp HTTP/1.1\r\nHost: {}\r\nContent-Length: {}\r\nContent-Type: application/json\r\n\r\n{}",
        addr, req_body_str.len(), req_body_str
    );
    use std::io::Write;
    stream.write_all(request_str.as_bytes()).unwrap();
    
    for _ in 0..50 {
        harness.app.tick();
        std::thread::sleep(std::time::Duration::from_millis(10));
    }
    
    assert_eq!(harness.app.tabs[0].panes().len(), 1);
    assert_eq!(harness.app.tabs[0].active_pane_id, initial_pane_id);
}

#[test]
fn test_mcp_server_focus_pane() {
    let temp_dir = std::env::temp_dir().join(format!("splash_mcp_focus_{}", std::process::id()));
    let _ = std::fs::create_dir_all(&temp_dir);
    let empty_tree = FileTree::new(&temp_dir).unwrap();
    
    let config = HarnessConfig {
        command: "sh".to_string(),
        args: vec![],
    };
    
    let mut harness = TestHarness::with_file_tree(80, 24, config, empty_tree);
    
    let original_pane_id = harness.app.tabs[0].active_pane_id;
    harness.app.split_active_pane(splash::app::SplitDirection::Vertical, splash::app::PaneContent::File(splash::app::FileTab { path: std::path::PathBuf::from("dummy"), content: String::new(), scroll_offset: 0 }));
    let new_pane_id = harness.app.tabs[0].active_pane_id;
    assert_ne!(original_pane_id, new_pane_id);
    
    let mcp_url = harness.app.mcp_url.clone().unwrap();
    let request_body = serde_json::json!({
        "jsonrpc": "2.0",
        "method": "tools/call",
        "params": {
            "name": "focus_pane",
            "arguments": {
                "pane_id": original_pane_id
            }
        },
        "id": 4
    });
    
    let addr = mcp_url.replace("http://", "");
    let mut stream = std::net::TcpStream::connect(&addr).unwrap();
    let req_body_str = serde_json::to_string(&request_body).unwrap();
    let request_str = format!(
        "POST /mcp HTTP/1.1\r\nHost: {}\r\nContent-Length: {}\r\nContent-Type: application/json\r\n\r\n{}",
        addr, req_body_str.len(), req_body_str
    );
    use std::io::Write;
    stream.write_all(request_str.as_bytes()).unwrap();
    
    for _ in 0..50 {
        harness.app.tick();
        std::thread::sleep(std::time::Duration::from_millis(10));
    }
    
    assert_eq!(harness.app.tabs[0].active_pane_id, original_pane_id);
    assert_eq!(harness.app.focus, splash::app::Focus::MainPane);
}

#[test]
fn test_mcp_server_switch_tab() {
    let temp_dir = std::env::temp_dir().join(format!("splash_mcp_tab_{}", std::process::id()));
    let _ = std::fs::create_dir_all(&temp_dir);
    let empty_tree = FileTree::new(&temp_dir).unwrap();
    
    let config = HarnessConfig {
        command: "sh".to_string(),
        args: vec![],
    };
    
    let mut harness = TestHarness::with_file_tree(80, 24, config, empty_tree);
    
    harness.app.tabs.push(splash::app::Tab::new(splash::app::PaneContent::File(splash::app::FileTab { path: std::path::PathBuf::from("dummy"), content: String::new(), scroll_offset: 0 })));
    harness.app.active_tab_index = 1;
    
    let mcp_url = harness.app.mcp_url.clone().unwrap();
    let request_body = serde_json::json!({
        "jsonrpc": "2.0",
        "method": "tools/call",
        "params": {
            "name": "switch_tab",
            "arguments": {
                "tab_index": 0
            }
        },
        "id": 5
    });
    
    let addr = mcp_url.replace("http://", "");
    let mut stream = std::net::TcpStream::connect(&addr).unwrap();
    let req_body_str = serde_json::to_string(&request_body).unwrap();
    let request_str = format!(
        "POST /mcp HTTP/1.1\r\nHost: {}\r\nContent-Length: {}\r\nContent-Type: application/json\r\n\r\n{}",
        addr, req_body_str.len(), req_body_str
    );
    use std::io::Write;
    stream.write_all(request_str.as_bytes()).unwrap();
    
    for _ in 0..50 {
        harness.app.tick();
        std::thread::sleep(std::time::Duration::from_millis(10));
    }
    
    assert_eq!(harness.app.active_tab_index, 0);
}
