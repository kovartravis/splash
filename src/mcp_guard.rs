use std::fs;
use std::path::PathBuf;
use std::sync::Mutex;
use serde_json::{json, Value};

#[derive(Debug, Clone)]
struct GuardCleanupEntry {
    config_path: PathBuf,
    server_name: String,
    created_file: bool,
}

impl GuardCleanupEntry {
    fn cleanup(&self) {
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

static REGISTERED_GUARDS: Mutex<Vec<GuardCleanupEntry>> = Mutex::new(Vec::new());

#[derive(Debug)]
pub struct McpConfigGuard {
    pub config_path: PathBuf,
    pub server_name: String,
    pub created_file: bool,
}

pub fn claude_config_path() -> PathBuf {
    if let Ok(override_path) = std::env::var("SPLASH_CLAUDE_CONFIG_PATH") {
        return PathBuf::from(override_path);
    }
    let home = std::env::var("HOME")
        .or_else(|_| std::env::var("USERPROFILE"))
        .unwrap_or_else(|_| ".".to_string());
    let home_path = std::path::Path::new(&home);

    #[cfg(target_os = "macos")]
    {
        home_path.join("Library/Application Support/Claude/claude_desktop_config.json")
    }
    #[cfg(target_os = "windows")]
    {
        if let Ok(appdata) = std::env::var("APPDATA") {
            std::path::Path::new(&appdata).join("Claude/claude_desktop_config.json")
        } else {
            home_path.join("AppData/Roaming/Claude/claude_desktop_config.json")
        }
    }
    #[cfg(not(any(target_os = "macos", target_os = "windows")))]
    {
        if let Ok(xdg) = std::env::var("XDG_CONFIG_HOME") {
            std::path::Path::new(&xdg).join("Claude/claude_desktop_config.json")
        } else {
            home_path.join(".config/Claude/claude_desktop_config.json")
        }
    }
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

        if let Ok(mut lock) = REGISTERED_GUARDS.lock() {
            lock.push(GuardCleanupEntry {
                config_path: config_path.clone(),
                server_name: server_name.clone(),
                created_file,
            });
        }

        Ok(Self {
            config_path,
            server_name,
            created_file,
        })
    }

    pub fn cleanup_all() {
        let entries = if let Ok(mut lock) = REGISTERED_GUARDS.lock() {
            std::mem::take(&mut *lock)
        } else {
            Vec::new()
        };

        for entry in entries {
            entry.cleanup();
        }
    }
}

pub fn setup_panic_hook() {
    let default_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        McpConfigGuard::cleanup_all();
        default_hook(info);
    }));
}

pub fn setup_signal_handlers() {
    let _ = ctrlc::set_handler(move || {
        McpConfigGuard::cleanup_all();
        std::process::exit(130);
    });
}

pub fn install_signal_and_panic_hooks() {
    setup_panic_hook();
    setup_signal_handlers();
}

impl Drop for McpConfigGuard {
    fn drop(&mut self) {
        GuardCleanupEntry {
            config_path: self.config_path.clone(),
            server_name: self.server_name.clone(),
            created_file: self.created_file,
        }
        .cleanup();

        if let Ok(mut lock) = REGISTERED_GUARDS.lock() {
            lock.retain(|entry| !(entry.config_path == self.config_path && entry.server_name == self.server_name));
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

    #[test]
    fn test_mcp_config_guard_cleanup_all_triggers_disk_cleanup() {
        let temp_dir = std::env::temp_dir().join("splash_test_mcp_guard_cleanup_all");
        let _ = fs::remove_dir_all(&temp_dir);
        fs::create_dir_all(&temp_dir).unwrap();

        let config_path = temp_dir.join("mcp_config.json");
        let guard = McpConfigGuard::register(&config_path, "splash", "http://127.0.0.1:9999")
            .expect("Failed to register McpConfigGuard");

        assert!(config_path.exists());

        // Call cleanup_all without dropping guard manually
        McpConfigGuard::cleanup_all();

        // Disk cleanup should have been triggered
        assert!(!config_path.exists());

        // Retain guard in scope so we are certain cleanup_all removed it, not stack drop
        let _ = guard;
        let _ = fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_panic_hook_triggers_mcp_config_cleanup() {
        let temp_dir = std::env::temp_dir().join("splash_test_mcp_guard_panic");
        let _ = fs::remove_dir_all(&temp_dir);
        fs::create_dir_all(&temp_dir).unwrap();

        let config_path = temp_dir.join("mcp_config.json");

        // Install panic hook
        setup_panic_hook();

        let path_clone = config_path.clone();
        let handle = std::thread::spawn(move || {
            let guard = McpConfigGuard::register(&path_clone, "splash", "http://127.0.0.1:9999")
                .expect("Failed to register McpConfigGuard");
            assert!(path_clone.exists());
            
            // Leak guard so stack unwinding / drop doesn't clean it up, simulating process abort/panic
            std::mem::forget(guard);
            panic!("Simulated panic in test thread");
        });

        let _ = handle.join();

        // Disk cleanup should have been triggered by panic hook
        assert!(!config_path.exists());

        let _ = fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_claude_config_path_default_and_override() {
        let default_path = claude_config_path();
        assert!(default_path.to_string_lossy().contains("claude_desktop_config.json"));

        let custom = "/tmp/custom_claude_config.json";
        unsafe { std::env::set_var("SPLASH_CLAUDE_CONFIG_PATH", custom); }
        assert_eq!(claude_config_path(), PathBuf::from(custom));
        unsafe { std::env::remove_var("SPLASH_CLAUDE_CONFIG_PATH"); }
    }
}
