use std::fs;
use std::path::PathBuf;
use serde_json::{json, Value};

#[derive(Debug)]
pub struct McpConfigGuard {
    pub config_path: PathBuf,
    pub server_name: String,
    pub created_file: bool,
}

impl McpConfigGuard {
    pub fn register(
        config_path: impl Into<PathBuf>,
        server_name: impl Into<String>,
        mcp_url: &str,
    ) -> Result<Self, String> {
        let config_path = config_path.into();
        let server_name = server_name.into();
        let created_file = !config_path.exists();

        let mut root: Value = if config_path.exists() {
            let content = fs::read_to_string(&config_path)
                .map_err(|e| format!("Failed to read MCP config at {:?}: {}", config_path, e))?;
            serde_json::from_str(&content).unwrap_or_else(|_| json!({}))
        } else {
            json!({})
        };

        if !root.is_object() {
            root = json!({});
        }

        let mcp_servers = root
            .as_object_mut()
            .unwrap()
            .entry("mcpServers")
            .or_insert_with(|| json!({}));

        if !mcp_servers.is_object() {
            *mcp_servers = json!({});
        }

        mcp_servers.as_object_mut().unwrap().insert(
            server_name.clone(),
            json!({
                "url": mcp_url,
                "type": "sse"
            }),
        );

        if let Some(parent) = config_path.parent() {
            if !parent.as_os_str().is_empty() && !parent.exists() {
                fs::create_dir_all(parent)
                    .map_err(|e| format!("Failed to create directory {:?}: {}", parent, e))?;
            }
        }

        let serialized = serde_json::to_string_pretty(&root)
            .map_err(|e| format!("Failed to serialize MCP config: {}", e))?;

        fs::write(&config_path, serialized)
            .map_err(|e| format!("Failed to write MCP config to {:?}: {}", config_path, e))?;

        Ok(Self {
            config_path,
            server_name,
            created_file,
        })
    }
}

impl Drop for McpConfigGuard {
    fn drop(&mut self) {
        if self.created_file {
            let _ = fs::remove_file(&self.config_path);
        } else if self.config_path.exists() {
            if let Ok(content) = fs::read_to_string(&self.config_path) {
                if let Ok(mut root) = serde_json::from_str::<Value>(&content) {
                    if let Some(mcp_servers) = root.get_mut("mcpServers").and_then(|v| v.as_object_mut()) {
                        mcp_servers.remove(&self.server_name);
                    }
                    if let Ok(serialized) = serde_json::to_string_pretty(&root) {
                        let _ = fs::write(&self.config_path, serialized);
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn test_mcp_config_guard_creates_file_and_deletes_on_drop() {
        let temp_dir = std::env::temp_dir().join("splash_test_mcp_guard_1");
        let _ = fs::remove_dir_all(&temp_dir);
        fs::create_dir_all(&temp_dir).unwrap();

        let config_path = temp_dir.join("mcp_config.json");
        assert!(!config_path.exists());

        {
            let guard = McpConfigGuard::register(&config_path, "splash", "http://127.0.0.1:9999")
                .expect("Failed to register McpConfigGuard");

            assert!(guard.created_file);
            assert!(config_path.exists());

            let content = fs::read_to_string(&config_path).unwrap();
            let json: serde_json::Value = serde_json::from_str(&content).unwrap();
            assert_eq!(
                json["mcpServers"]["splash"]["url"],
                "http://127.0.0.1:9999"
            );
            assert_eq!(
                json["mcpServers"]["splash"]["type"],
                "sse"
            );
        }

        // After drop, file created by Splash should be removed
        assert!(!config_path.exists());

        let _ = fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_mcp_config_guard_preserves_pre_existing_config() {
        let temp_dir = std::env::temp_dir().join("splash_test_mcp_guard_2");
        let _ = fs::remove_dir_all(&temp_dir);
        fs::create_dir_all(&temp_dir).unwrap();

        let config_path = temp_dir.join("mcp_config.json");
        let initial_json = json!({
            "mcpServers": {
                "other_server": {
                    "url": "http://127.0.0.1:8080",
                    "type": "sse"
                }
            }
        });
        fs::write(&config_path, serde_json::to_string_pretty(&initial_json).unwrap()).unwrap();

        {
            let guard = McpConfigGuard::register(&config_path, "splash", "http://127.0.0.1:9999")
                .expect("Failed to register McpConfigGuard");

            assert!(!guard.created_file);
            assert!(config_path.exists());

            let content = fs::read_to_string(&config_path).unwrap();
            let json: serde_json::Value = serde_json::from_str(&content).unwrap();
            assert_eq!(
                json["mcpServers"]["splash"]["url"],
                "http://127.0.0.1:9999"
            );
            assert_eq!(
                json["mcpServers"]["other_server"]["url"],
                "http://127.0.0.1:8080"
            );
        }

        // After drop, pre-existing file remains, "splash" removed, "other_server" preserved
        assert!(config_path.exists());
        let content = fs::read_to_string(&config_path).unwrap();
        let json: serde_json::Value = serde_json::from_str(&content).unwrap();
        assert!(json["mcpServers"]["splash"].is_null());
        assert_eq!(
            json["mcpServers"]["other_server"]["url"],
            "http://127.0.0.1:8080"
        );

        let _ = fs::remove_dir_all(&temp_dir);
    }
}

