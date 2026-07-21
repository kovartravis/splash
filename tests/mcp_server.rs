
use splash::pty::HarnessConfig;
use splash::testing::TestHarness;
use splash::tree::FileTree;
use serde_json::Value;

// ── Helpers ───────────────────────────────────────────────────────────────────

/// Send a JSON-RPC request to the MCP server and return the parsed JSON response.
///
/// Phase 1 — connect + write on the **main thread**, so the request bytes are
///            in the kernel's TCP receive buffer before the first tick().
/// Phase 2 — blocking `read_to_string` on a **background thread**: it just
///            waits without burning CPU until the server sends a response.
/// Phase 3 — main thread ticks the app at 20 ms intervals (giving tiny_http's
///            internal acceptor threads CPU time) and polls a channel for the
///            response from the background thread.
fn mcp_roundtrip(harness: &mut TestHarness, mcp_url: &str, body: serde_json::Value) -> Value {
    let raw = mcp_roundtrip_raw(harness, mcp_url, body);
    serde_json::from_str(&raw).expect("MCP response body is not valid JSON")
}

/// Like `mcp_roundtrip` but returns the raw HTTP response (status + headers + body).
fn mcp_roundtrip_raw(
    harness: &mut TestHarness,
    mcp_url: &str,
    body: serde_json::Value,
) -> String {
    let mut child = std::process::Command::new(env!("CARGO_BIN_EXE_splash"))
        .arg("mcp-proxy")
        .arg(mcp_url)
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .spawn()
        .expect("Failed to spawn mcp-proxy");

    let mut stdin = child.stdin.take().expect("Failed to open proxy stdin");
    let mut stdout = child.stdout.take().expect("Failed to open proxy stdout");

    // Phase 1: send JSONL request on the main thread.
    let body_str = serde_json::to_string(&body).unwrap();
    let request = format!("{}\n", body_str);
    use std::io::Write;
    stdin.write_all(request.as_bytes()).unwrap();
    stdin.flush().unwrap();

    // Phase 2: blocking read on a background thread.
    let (tx, rx) = std::sync::mpsc::channel::<String>();
    std::thread::spawn(move || {
        use std::io::BufRead;
        let mut reader = std::io::BufReader::new(stdout);
        let mut response = String::new();
        // read_line returns when the proxy responds with JSONL newline
        let _ = reader.read_line(&mut response);
        let _ = tx.send(response);
    });

    // Phase 3: tick every 20 ms until the response arrives.
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(12);
    loop {
        match rx.try_recv() {
            Ok(response) => return response,
            Err(std::sync::mpsc::TryRecvError::Empty) => {}
            Err(std::sync::mpsc::TryRecvError::Disconnected) => {
                panic!("MCP reader thread disconnected before sending a response")
            }
        }

        assert!(
            std::time::Instant::now() < deadline,
            "Timed out (12 s) waiting for MCP response from {}",
            mcp_url
        );

        harness.app.tick();
        // 20 ms sleep gives tiny_http's internal acceptor threads CPU time to
        // accept the connection, read the request, and enqueue it for try_recv().
        std::thread::sleep(std::time::Duration::from_millis(20));
    }
}

fn make_harness() -> (TestHarness, String) {
    // Static counter so each parallel test gets a unique temp-dir name within
    // the same process (process::id() alone isn't enough when tests run in
    // parallel threads).
    use std::sync::atomic::{AtomicU64, Ordering};
    static COUNTER: AtomicU64 = AtomicU64::new(0);
    let n = COUNTER.fetch_add(1, Ordering::Relaxed);

    let temp_dir = std::env::temp_dir()
        .join(format!("splash_mcp_{}_{}", std::process::id(), n));
    let _ = std::fs::create_dir_all(&temp_dir);
    let tree = splash::tree::FileTree::new(&temp_dir).unwrap();
    let config = HarnessConfig {
        command: "sh".to_string(),
        args: vec!["-c".to_string(), "true".to_string()],
    };
    let harness = TestHarness::with_file_tree(80, 24, config, tree);
    let mcp_url = harness
        .app
        .mcp_url
        .clone()
        .expect("mcp_url not set on SplashApp");
    (harness, mcp_url)
}

// ── MCP lifecycle tests ───────────────────────────────────────────────────────

#[test]
fn test_mcp_initialize_handshake() {
    let (mut harness, mcp_url) = make_harness();

    let res = mcp_roundtrip(
        &mut harness,
        &mcp_url,
        serde_json::json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "initialize",
            "params": {
                "protocolVersion": "2024-11-05",
                "capabilities": {},
                "clientInfo": { "name": "test-client", "version": "0.0.1" }
            }
        }),
    );

    assert_eq!(res["jsonrpc"], "2.0");
    assert_eq!(res["id"], 1);
    let result = &res["result"];
    assert!(!result.is_null(), "expected a result field");
    assert_eq!(result["serverInfo"]["name"], "splash");
    assert!(
        result.get("protocolVersion").is_some(),
        "initialize result must contain protocolVersion"
    );
    assert!(
        result.get("capabilities").is_some(),
        "initialize result must contain capabilities"
    );
}

#[test]
fn test_mcp_tools_list() {
    let (mut harness, mcp_url) = make_harness();

    let res = mcp_roundtrip(
        &mut harness,
        &mcp_url,
        serde_json::json!({
            "jsonrpc": "2.0",
            "id": 2,
            "method": "tools/list",
            "params": {}
        }),
    );

    assert_eq!(res["jsonrpc"], "2.0");
    let tools = res["result"]["tools"]
        .as_array()
        .expect("tools/list result must have a 'tools' array");

    let tool_names: Vec<&str> = tools
        .iter()
        .map(|t| t["name"].as_str().expect("tool must have a name"))
        .collect();

    for expected in &["list_layout", "open_file", "close_pane", "focus_pane", "switch_tab"] {
        assert!(
            tool_names.contains(expected),
            "tools/list missing tool '{}'",
            expected
        );
    }

    for tool in tools {
        assert!(
            tool.get("inputSchema").is_some(),
            "tool '{}' is missing inputSchema",
            tool["name"]
        );
    }
}

#[test]
fn test_mcp_notifications_initialized() {
    // notifications/initialized is a one-way notification — the server must not
    // hang and should return 2xx (we use 204 No Content).
    let (mut harness, mcp_url) = make_harness();
    let addr = mcp_url.replace("http://", "");
    
    use std::io::{Read, Write};
    let mut stream = std::net::TcpStream::connect(&addr).expect("Failed to connect");
    
    let body = serde_json::json!({
        "jsonrpc": "2.0",
        "method": "notifications/initialized"
    });
    let body_str = serde_json::to_string(&body).unwrap();
    let request = format!(
        "POST /mcp HTTP/1.1\r\nHost: {}\r\nContent-Length: {}\r\nContent-Type: application/json\r\n\r\n{}",
        addr,
        body_str.len(),
        body_str
    );
    stream.write_all(request.as_bytes()).unwrap();
    
    let mut response = String::new();
    stream.set_read_timeout(Some(std::time::Duration::from_millis(100))).unwrap();
    for _ in 0..50 {
        harness.app.tick();
        if stream.read_to_string(&mut response).is_ok() || !response.is_empty() {
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(20));
    }

    assert!(
        response.starts_with("HTTP/1.1 204") || response.starts_with("HTTP/1.1 200"),
        "Expected 2xx response for notifications/initialized, got: {}",
        response.lines().next().unwrap_or("")
    );
}

// ── tools/call tests ──────────────────────────────────────────────────────────

#[test]
fn test_mcp_server_and_list_layout() {
    let temp_dir = std::env::temp_dir().join(format!("splash_mcp_ll_{}", std::process::id()));
    let _ = std::fs::create_dir_all(&temp_dir);
    let empty_tree = FileTree::new(&temp_dir).unwrap();

    let config = HarnessConfig {
        command: "sh".to_string(),
        args: vec!["-c".to_string(), "echo MCP_URL=$SPLASH_MCP_URL".to_string()],
    };

    let mut harness = TestHarness::with_file_tree(80, 24, config, empty_tree);
    let mcp_url = harness.app.mcp_url.clone().expect("SPLASH_MCP_URL was not found on app");
    assert!(mcp_url.starts_with("http://127.0.0.1:"));

    let res = mcp_roundtrip(
        &mut harness,
        &mcp_url,
        serde_json::json!({
            "jsonrpc": "2.0",
            "method": "tools/call",
            "params": { "name": "list_layout", "arguments": {} },
            "id": 1
        }),
    );

    let result = &res["result"];
    assert!(!result.is_null(), "Expected a result in the JSON-RPC response");

    let content = result["content"].as_array().expect("Result should have content array");
    let text_content = content[0]["text"].as_str().expect("Content should have text");

    let parsed_layout: Value =
        serde_json::from_str(text_content).expect("Text content should be JSON layout");
    assert_eq!(parsed_layout["tabs"].as_array().unwrap().len(), 1);

    let first_tab = &parsed_layout["tabs"][0];
    assert_eq!(first_tab["active_pane_id"].as_u64().unwrap(), 0);
}

#[test]
fn test_mcp_server_open_file() {
    let temp_dir = std::env::temp_dir().join(format!("splash_mcp_open_{}", std::process::id()));
    let _ = std::fs::create_dir_all(&temp_dir);
    std::fs::write(temp_dir.join("test_file.rs"), "fn test() {}").unwrap();
    let empty_tree = FileTree::new(&temp_dir).unwrap();

    let config = HarnessConfig {
        command: "sh".to_string(),
        args: vec!["-c".to_string(), "echo MCP_URL=$SPLASH_MCP_URL".to_string()],
    };

    let mut harness = TestHarness::with_file_tree(80, 24, config, empty_tree);
    let mcp_url = harness.app.mcp_url.clone().expect("SPLASH_MCP_URL was not found on app");

    mcp_roundtrip(
        &mut harness,
        &mcp_url,
        serde_json::json!({
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
        }),
    );

    assert_eq!(harness.app.tabs.len(), 1);
    assert_eq!(harness.app.tabs[0].panes().len(), 2);

    let active_pane_id = harness.app.tabs[0].active_pane_id;
    let active_pane = harness.app.tabs[0]
        .panes()
        .into_iter()
        .find(|p| p.id == active_pane_id)
        .unwrap();
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

    // Split pane to have 2 panes.
    harness.app.split_active_pane(
        splash::app::SplitDirection::Vertical,
        splash::app::PaneContent::File(
            splash::app::FileTab::open(temp_dir.join("dummy")).unwrap_or_else(|_| {
                splash::app::FileTab {
                    path: std::path::PathBuf::from("dummy"),
                    content: String::new(),
                    scroll_offset: 0,
                }
            }),
        ),
    );
    assert_eq!(harness.app.tabs[0].panes().len(), 2);
    let pane_id_to_close = harness.app.tabs[0].active_pane_id;
    let initial_pane_id = harness.app.tabs[0]
        .panes()
        .iter()
        .find(|p| p.id != pane_id_to_close)
        .unwrap()
        .id;

    let mcp_url = harness.app.mcp_url.clone().unwrap();
    mcp_roundtrip(
        &mut harness,
        &mcp_url,
        serde_json::json!({
            "jsonrpc": "2.0",
            "method": "tools/call",
            "params": {
                "name": "close_pane",
                "arguments": { "pane_id": pane_id_to_close }
            },
            "id": 3
        }),
    );

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
    harness.app.split_active_pane(
        splash::app::SplitDirection::Vertical,
        splash::app::PaneContent::File(splash::app::FileTab {
            path: std::path::PathBuf::from("dummy"),
            content: String::new(),
            scroll_offset: 0,
        }),
    );
    let new_pane_id = harness.app.tabs[0].active_pane_id;
    assert_ne!(original_pane_id, new_pane_id);

    let mcp_url = harness.app.mcp_url.clone().unwrap();
    mcp_roundtrip(
        &mut harness,
        &mcp_url,
        serde_json::json!({
            "jsonrpc": "2.0",
            "method": "tools/call",
            "params": {
                "name": "focus_pane",
                "arguments": { "pane_id": original_pane_id }
            },
            "id": 4
        }),
    );

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
    harness.app.tabs.push(splash::app::Tab::new(
        splash::app::PaneContent::File(splash::app::FileTab {
            path: std::path::PathBuf::from("dummy"),
            content: String::new(),
            scroll_offset: 0,
        }),
    ));
    harness.app.active_tab_index = 1;

    let mcp_url = harness.app.mcp_url.clone().unwrap();
    mcp_roundtrip(
        &mut harness,
        &mcp_url,
        serde_json::json!({
            "jsonrpc": "2.0",
            "method": "tools/call",
            "params": {
                "name": "switch_tab",
                "arguments": { "tab_index": 0 }
            },
            "id": 5
        }),
    );

    assert_eq!(harness.app.active_tab_index, 0);
}
