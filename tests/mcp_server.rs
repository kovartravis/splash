use splash::pty::HarnessConfig;
use splash::testing::TestHarness;
use splash::tree::FileTree;
use serde_json::Value;

#[test]
fn test_mcp_server_and_list_layout() {
    let temp_dir = std::env::temp_dir().join(format!("splash_mcp_{}", std::process::id()));
    let _ = std::fs::create_dir_all(&temp_dir);
    let empty_tree = FileTree::new(&temp_dir).unwrap();
    
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
    
    stream.set_read_timeout(Some(std::time::Duration::from_secs(1))).unwrap();
    
    let mut response = String::new();
    let _ = stream.read_to_string(&mut response);
    
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
