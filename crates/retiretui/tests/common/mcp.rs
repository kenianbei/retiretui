//! A JSON-RPC client speaking MCP to the built binary over stdio.

use std::io::{BufRead, BufReader, Write};
use std::path::Path;
use std::process::{Child, ChildStdin, ChildStdout, Command, Stdio};

use serde_json::{Value, json};

const PROTOCOL_VERSION: &str = "2025-06-18";

pub struct McpClient {
    child: Child,
    stdin: ChildStdin,
    stdout: BufReader<ChildStdout>,
    next_id: u64,
}

impl McpClient {
    pub fn spawn(root: &Path) -> Self {
        let mut child = Command::new(env!("CARGO_BIN_EXE_retiretui"))
            .args(["mcp", "--dir", root.to_str().unwrap()])
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .spawn()
            .unwrap();
        let stdin = child.stdin.take().unwrap();
        let stdout = BufReader::new(child.stdout.take().unwrap());
        let mut client = Self {
            child,
            stdin,
            stdout,
            next_id: 0,
        };
        let init = client.request(
            "initialize",
            json!({
                "protocolVersion": PROTOCOL_VERSION,
                "capabilities": {},
                "clientInfo": {"name": "test", "version": "0"},
            }),
        );
        assert!(init.get("serverInfo").is_some(), "{init}");
        client.send(&json!({
            "jsonrpc": "2.0",
            "method": "notifications/initialized",
        }));
        client
    }

    fn send(&mut self, message: &Value) {
        let mut line = message.to_string();
        line.push('\n');
        self.stdin.write_all(line.as_bytes()).unwrap();
        self.stdin.flush().unwrap();
    }

    /// Sends one request and returns its `result`, panicking on a JSON-RPC
    /// error.
    pub fn request(&mut self, method: &str, params: Value) -> Value {
        self.next_id += 1;
        let id = self.next_id;
        self.send(&json!({
            "jsonrpc": "2.0",
            "id": id,
            "method": method,
            "params": params,
        }));
        loop {
            let mut line = String::new();
            self.stdout.read_line(&mut line).unwrap();
            assert!(!line.is_empty(), "server closed stdout");
            let message: Value = serde_json::from_str(&line).unwrap();
            if message.get("id").and_then(Value::as_u64) != Some(id) {
                continue;
            }
            assert!(message.get("error").is_none(), "{method} failed: {message}");
            return message["result"].clone();
        }
    }

    /// Calls a tool expecting success and returns its structured content.
    pub fn call(&mut self, tool: &str, arguments: Value) -> Value {
        let outcome = self.call_raw(tool, arguments);
        assert_ne!(
            outcome.get("isError").and_then(Value::as_bool),
            Some(true),
            "{tool} errored: {outcome}"
        );
        outcome["structuredContent"].clone()
    }

    /// Calls a tool expecting refusal and returns the error text.
    pub fn call_expecting_error(&mut self, tool: &str, arguments: Value) -> String {
        let outcome = self.call_raw(tool, arguments);
        assert_eq!(
            outcome.get("isError").and_then(Value::as_bool),
            Some(true),
            "{tool} unexpectedly succeeded: {outcome}"
        );
        outcome["content"][0]["text"].as_str().unwrap().to_owned()
    }

    pub fn call_raw(&mut self, tool: &str, arguments: Value) -> Value {
        self.request("tools/call", json!({"name": tool, "arguments": arguments}))
    }
}

impl Drop for McpClient {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}
