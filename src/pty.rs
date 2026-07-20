use std::env;
use std::io::{Read, Write};
use std::sync::mpsc::{channel, Receiver};
use std::thread;

use portable_pty::{native_pty_system, CommandBuilder, PtyPair, PtySize};

#[derive(Debug, PartialEq, Clone)]
pub struct HarnessConfig {
    pub command: String,
    pub args: Vec<String>,
}

pub fn parse_args(args: &[String]) -> Result<HarnessConfig, String> {
    if args.len() < 2 {
        return Err("Usage: splash <harness-command>".to_string());
    }
    Ok(HarnessConfig {
        command: args[1].clone(),
        args: args[2..].to_vec(),
    })
}

pub struct PtyHarness {
    pub pty_pair: PtyPair,
    pub writer: Box<dyn Write + Send>,
    pub output_rx: Receiver<String>,
    #[allow(dead_code)]
    pub child: Box<dyn portable_pty::Child + Send + Sync>,
}

impl PtyHarness {
    pub fn spawn(config: &HarnessConfig, rows: u16, cols: u16, mcp_url: Option<&str>) -> Result<Self, String> {
        let pty_system = native_pty_system();
        let pty_pair = pty_system
            .openpty(PtySize {
                rows,
                cols,
                pixel_width: 0,
                pixel_height: 0,
            })
            .map_err(|e| format!("Failed to open PTY: {}", e))?;

        let mut cmd = CommandBuilder::new(&config.command);
        let mut final_args = config.args.clone();

        if let Some(url) = mcp_url {
            cmd.env("SPLASH_MCP_URL", url);
            
            let command_name = std::path::Path::new(&config.command)
                .file_name()
                .and_then(|s| s.to_str())
                .unwrap_or("");

            match command_name {
                "agy" => {
                    final_args.push("--mcp-server".to_string());
                    final_args.push(url.to_string());
                }
                "claude" => {
                    let tmp_path = format!("/tmp/splash_claude_mcp_{}.json", std::process::id());
                    // Create a valid Claude MCP config JSON
                    let mcp_config = format!(
                        r#"{{"mcpServers":{{"splash":{{"command":"curl","args":["-X","POST","{}"]}}}}}}"#,
                        url
                    );
                    let _ = std::fs::write(&tmp_path, mcp_config);
                    final_args.push("--mcp-config".to_string());
                    final_args.push(tmp_path);
                }
                _ => {}
            }
        }
        
        cmd.args(&final_args);
        
        if let Ok(cwd) = env::current_dir() {
            cmd.cwd(cwd);
        }

        let child = pty_pair
            .slave
            .spawn_command(cmd)
            .map_err(|e| format!("Failed to spawn command '{}': {}", config.command, e))?;

        let writer = pty_pair
            .master
            .take_writer()
            .map_err(|e| format!("Failed to take PTY writer: {}", e))?;

        let mut reader = pty_pair
            .master
            .try_clone_reader()
            .map_err(|e| format!("Failed to clone PTY reader: {}", e))?;

        let (tx, rx) = channel();

        thread::spawn(move || {
            let mut buf = [0u8; 1024];
            loop {
                match reader.read(&mut buf) {
                    Ok(0) => break,
                    Ok(n) => {
                        let text = String::from_utf8_lossy(&buf[..n]).to_string();
                        if tx.send(text).is_err() {
                            break;
                        }
                    }
                    Err(_) => break,
                }
            }
        });

        Ok(PtyHarness {
            pty_pair,
            writer,
            output_rx: rx,
            child,
        })
    }

    pub fn resize(&self, rows: u16, cols: u16) {
        let _ = self.pty_pair.master.resize(PtySize {
            rows,
            cols,
            pixel_width: 0,
            pixel_height: 0,
        });
    }

    pub fn write(&mut self, bytes: &[u8]) {
        let _ = self.writer.write_all(bytes);
        let _ = self.writer.flush();
    }

    pub fn kill(&mut self) {
        let _ = self.child.kill();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_args_valid() {
        let args = vec!["splash".to_string(), "bash".to_string()];
        let config = parse_args(&args).unwrap();
        assert_eq!(config.command, "bash");
        assert!(config.args.is_empty());
    }

    #[test]
    fn test_parse_args_with_cmd_args() {
        let args = vec![
            "splash".to_string(),
            "echo".to_string(),
            "hello".to_string(),
            "world".to_string(),
        ];
        let config = parse_args(&args).unwrap();
        assert_eq!(config.command, "echo");
        assert_eq!(config.args, vec!["hello", "world"]);
    }

    #[test]
    fn test_parse_args_missing() {
        let args = vec!["splash".to_string()];
        let err = parse_args(&args).unwrap_err();
        assert!(err.contains("Usage: splash <harness-command>"));
    }

    #[test]
    fn test_pty_harness_spawn_and_read() {
        let config = HarnessConfig {
            command: "echo".to_string(),
            args: vec!["hello_splash".to_string()],
        };
        let harness = PtyHarness::spawn(&config, 24, 80, None).unwrap();

        // Wait for output from reader thread
        let mut output = String::new();
        let start = std::time::Instant::now();
        while start.elapsed() < std::time::Duration::from_secs(3) {
            if let Ok(chunk) = harness.output_rx.try_recv() {
                output.push_str(&chunk);
                if output.contains("hello_splash") {
                    break;
                }
            }
            std::thread::sleep(std::time::Duration::from_millis(50));
        }

        assert!(output.contains("hello_splash"));
    }

    #[test]
    fn test_pty_harness_spawn_agy_mcp_args() {
        let temp_dir = std::env::temp_dir();
        let mock_script = temp_dir.join(format!("agy_{}", std::process::id()));
        // rename it to agy so it matches
        let mock_script = temp_dir.join("agy");
        std::fs::write(&mock_script, "#!/bin/sh\necho \"$@\"").unwrap();
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(&mock_script, std::fs::Permissions::from_mode(0o755)).unwrap();
        }

        let config = HarnessConfig {
            command: mock_script.to_str().unwrap().to_string(),
            args: vec!["initial".to_string()],
        };
        
        let mcp_url = "http://127.0.0.1:9999";
        let harness = PtyHarness::spawn(&config, 24, 80, Some(mcp_url)).unwrap();
        
        let mut output = String::new();
        let start = std::time::Instant::now();
        while start.elapsed() < std::time::Duration::from_secs(3) {
            if let Ok(chunk) = harness.output_rx.try_recv() {
                output.push_str(&chunk);
                if output.contains("--mcp-server") {
                    break;
                }
            }
            std::thread::sleep(std::time::Duration::from_millis(50));
        }
        
        assert!(output.contains(&format!("initial --mcp-server {}", mcp_url)));
    }

    #[test]
    fn test_pty_harness_spawn_claude_mcp_args() {
        let temp_dir = std::env::temp_dir();
        let mock_script = temp_dir.join("claude");
        std::fs::write(&mock_script, "#!/bin/sh\necho \"$@\"").unwrap();
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(&mock_script, std::fs::Permissions::from_mode(0o755)).unwrap();
        }

        let config = HarnessConfig {
            command: mock_script.to_str().unwrap().to_string(),
            args: vec!["initial".to_string()],
        };
        
        let mcp_url = "http://127.0.0.1:8888";
        let harness = PtyHarness::spawn(&config, 24, 80, Some(mcp_url)).unwrap();
        
        let mut output = String::new();
        let start = std::time::Instant::now();
        while start.elapsed() < std::time::Duration::from_secs(3) {
            if let Ok(chunk) = harness.output_rx.try_recv() {
                output.push_str(&chunk);
                if output.contains("--mcp-config") {
                    break;
                }
            }
            std::thread::sleep(std::time::Duration::from_millis(50));
        }
        
        assert!(output.contains("initial --mcp-config "));
        assert!(output.contains("/tmp/splash_claude_mcp_"));
    }
}
