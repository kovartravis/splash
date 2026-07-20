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
        cmd.args(&config.args);
        
        if let Some(url) = mcp_url {
            cmd.env("SPLASH_MCP_URL", url);
        }
        
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
}
