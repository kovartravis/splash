import os

def resolve_app_rs():
    with open("src/app.rs", "r") as f:
        content = f.read()
    
    # Conflict 1
    c1 = """<<<<<<< HEAD
        if let Some(server) = &self.mcp_server {
=======
        if let Some(server) = self.mcp_server.clone() {
>>>>>>> origin/main"""
    content = content.replace(c1, "        if let Some(server) = self.mcp_server.clone() {")
    
    # Conflict 2
    c2 = """<<<<<<< HEAD
                            let response_str = response_json.to_string();
                            let response = tiny_http::Response::from_string(response_str)
                                .with_header(tiny_http::Header::from_bytes(&b"Content-Type"[..], &b"application/json"[..]).unwrap());
                            let _ = request.respond(response);
                            continue;
                        }
                    }
                }
                let _ = request.respond(tiny_http::Response::from_string("Not Found").with_status_code(404));
=======
                            let response_str = serde_json::to_string(&response_json).unwrap();
                            let response = tiny_http::Response::from_string(response_str)
                                .with_header(tiny_http::Header::from_bytes(&b"Content-Type"[..], &b"application/json"[..]).unwrap());
                            let _ = request.respond(response);
                        } else if json["method"] == "tools/call" && json["params"]["name"] == "open_file" {
                            let args = &json["params"]["arguments"];
                            let location = args["location"].as_str().unwrap_or("new_tab");
                            let file_path = args["file_path"].as_str().unwrap_or("");
                            
                            let mut success = true;
                            if let Ok(file_tab) = FileTab::open(std::path::Path::new(file_path)) {
                                match location {
                                    "split_right" => {
                                        self.split_active_pane(SplitDirection::Horizontal, PaneContent::File(file_tab));
                                    }
                                    "split_down" => {
                                        self.split_active_pane(SplitDirection::Vertical, PaneContent::File(file_tab));
                                    }
                                    "replace_active" => {
                                        if let Some(pane) = self.tabs[self.active_tab_index].active_pane_mut() {
                                            pane.content = PaneContent::File(file_tab);
                                        }
                                    }
                                    _ => { // "new_tab"
                                        self.tabs.push(Tab::new(PaneContent::File(file_tab)));
                                        self.active_tab_index = self.tabs.len() - 1;
                                    }
                                }
                            } else {
                                success = false;
                            }
                            
                            let text = if success { "File opened successfully".to_string() } else { "Failed to open file".to_string() };
                            let response_json = serde_json::json!({
                                "jsonrpc": "2.0",
                                "id": json["id"],
                                "result": {
                                    "content": [{
                                        "type": "text",
                                        "text": text
                                    }]
                                }
                            });
                            
                            let response_str = serde_json::to_string(&response_json).unwrap();
                            let response = tiny_http::Response::from_string(response_str)
                                .with_header(tiny_http::Header::from_bytes(&b"Content-Type"[..], &b"application/json"[..]).unwrap());
                            let _ = request.respond(response);
                        } else {
                            let _ = request.respond(tiny_http::Response::empty(404));
                        }
                    }
                }
>>>>>>> origin/main"""
    
    merged2 = """                            let response_str = serde_json::to_string(&response_json).unwrap();
                            let response = tiny_http::Response::from_string(response_str)
                                .with_header(tiny_http::Header::from_bytes(&b"Content-Type"[..], &b"application/json"[..]).unwrap());
                            let _ = request.respond(response);
                        } else if json["method"] == "tools/call" && json["params"]["name"] == "open_file" {
                            let args = &json["params"]["arguments"];
                            let location = args["location"].as_str().unwrap_or("new_tab");
                            let file_path = args["file_path"].as_str().unwrap_or("");
                            
                            let mut success = true;
                            if let Ok(file_tab) = FileTab::open(std::path::Path::new(file_path)) {
                                match location {
                                    "split_right" => {
                                        self.split_active_pane(SplitDirection::Horizontal, PaneContent::File(file_tab));
                                    }
                                    "split_down" => {
                                        self.split_active_pane(SplitDirection::Vertical, PaneContent::File(file_tab));
                                    }
                                    "replace_active" => {
                                        if let Some(pane) = self.tabs[self.active_tab_index].active_pane_mut() {
                                            pane.content = PaneContent::File(file_tab);
                                        }
                                    }
                                    _ => { // "new_tab"
                                        self.tabs.push(Tab::new(PaneContent::File(file_tab)));
                                        self.active_tab_index = self.tabs.len() - 1;
                                    }
                                }
                            } else {
                                success = false;
                            }
                            
                            let text = if success { "File opened successfully".to_string() } else { "Failed to open file".to_string() };
                            let response_json = serde_json::json!({
                                "jsonrpc": "2.0",
                                "id": json["id"],
                                "result": {
                                    "content": [{
                                        "type": "text",
                                        "text": text
                                    }]
                                }
                            });
                            
                            let response_str = serde_json::to_string(&response_json).unwrap();
                            let response = tiny_http::Response::from_string(response_str)
                                .with_header(tiny_http::Header::from_bytes(&b"Content-Type"[..], &b"application/json"[..]).unwrap());
                            let _ = request.respond(response);
                        } else {
                            let _ = request.respond(tiny_http::Response::empty(404));
                        }
                        continue;
                    }
                }
                let _ = request.respond(tiny_http::Response::empty(404));"""
    
    content = content.replace(c2, merged2)
    with open("src/app.rs", "w") as f:
        f.write(content)

def resolve_mcp_server_rs():
    with open("tests/mcp_server.rs", "r") as f:
        content = f.read()
        
    c1 = """<<<<<<< HEAD

=======
>>>>>>> origin/main"""
    content = content.replace(c1, "")
    
    c2 = """<<<<<<< HEAD
    // Command that prints the environment variable
=======
>>>>>>> origin/main"""
    content = content.replace(c2, "    // Command that prints the environment variable")
    
    c3 = """<<<<<<< HEAD
    let mcp_url = harness.app.mcp_url.clone().expect("SPLASH_MCP_URL was not found");
=======
    let mcp_url = harness.app.mcp_url.clone().expect("SPLASH_MCP_URL was not found on app");
>>>>>>> origin/main"""
    content = content.replace(c3, "    let mcp_url = harness.app.mcp_url.clone().expect(\"SPLASH_MCP_URL was not found on app\");")
    
    c4 = """<<<<<<< HEAD
    // Let the server process the request
=======
    // Pump app event loop to process the request
>>>>>>> origin/main"""
    content = content.replace(c4, "    // Pump app event loop to process the request")
    
    c5 = """<<<<<<< HEAD
    // Set read timeout
    stream.set_read_timeout(Some(std::time::Duration::from_secs(1))).unwrap();
    
    let mut response = String::new();
    let _ = stream.read_to_string(&mut response); // May return Error(WouldBlock) when socket is closed or timeout, but we just want whatever was read
=======
    stream.set_read_timeout(Some(std::time::Duration::from_secs(1))).unwrap();
    
    let mut response = String::new();
    let _ = stream.read_to_string(&mut response);
>>>>>>> origin/main"""
    
    m5 = """    // Set read timeout
    stream.set_read_timeout(Some(std::time::Duration::from_secs(1))).unwrap();
    
    let mut response = String::new();
    let _ = stream.read_to_string(&mut response); // May return Error(WouldBlock) when socket is closed or timeout, but we just want whatever was read"""
    content = content.replace(c5, m5)
    
    c6 = """<<<<<<< HEAD
=======

#[test]
fn test_mcp_server_open_file() {"""
    content = content.replace(c6, "\n#[test]\nfn test_mcp_server_open_file() {")
    
    content = content.replace(">>>>>>> origin/main\n", "")
    
    with open("tests/mcp_server.rs", "w") as f:
        f.write(content)

resolve_app_rs()
resolve_mcp_server_rs()
