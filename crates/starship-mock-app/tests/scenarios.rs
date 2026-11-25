use serde_json::Value;
use starship_mock_app::{FailureMode, Scenario};
use std::fs;
use std::io::{BufRead, BufReader, Write};
use std::os::unix::net::UnixStream;
use std::path::{Path, PathBuf};
use std::process::{Child, ChildStdin, ChildStdout, Command, Stdio};
use std::thread;
use std::time::{Duration, Instant};

const PROGRESS_METHOD: &str = "notifications/progress";

struct MockProcess {
    child: Child,
    stdin: ChildStdin,
    stdout: BufReader<ChildStdout>,
}

impl MockProcess {
    fn start(scenario: &str) -> Self {
        let path = scenario_path(scenario);
        let mut child = Command::new(env!("CARGO_BIN_EXE_starship-mock-app"))
            .arg("--mcp")
            .arg("--scenario")
            .arg(path)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
            .expect("spawn mock app");
        let stdin = child.stdin.take().expect("stdin pipe");
        let stdout = BufReader::new(child.stdout.take().expect("stdout pipe"));
        Self {
            child,
            stdin,
            stdout,
        }
    }

    fn call(&mut self, tool: &str, id: u64) {
        let request = serde_json::json!({
            "jsonrpc": "2.0",
            "id": id,
            "method": "tools/call",
            "params": {
                "name": tool,
                "arguments": {},
                "_meta": { "progressToken": "chaos-token" }
            }
        });
        writeln!(self.stdin, "{request}").expect("write request");
        self.stdin.flush().expect("flush request");
    }

    fn next_line(&mut self) -> String {
        let mut line = String::new();
        let read = self.stdout.read_line(&mut line).expect("read line");
        assert_ne!(read, 0, "mock app closed stdout unexpectedly");
        line.trim().to_owned()
    }

    fn lines_until_response(&mut self, id: u64) -> Vec<String> {
        let mut lines = Vec::new();
        for _ in 0..32 {
            let line = self.next_line();
            let is_response = serde_json::from_str::<Value>(&line)
                .is_ok_and(|value| value.get("id").and_then(Value::as_u64) == Some(id));
            lines.push(line);
            if is_response {
                return lines;
            }
        }
        panic!("no response with id {id}");
    }

    fn wait_for_exit(&mut self) -> i32 {
        self.child
            .wait()
            .expect("wait for exit")
            .code()
            .expect("exit code")
    }
}

impl Drop for MockProcess {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

fn scenarios_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("scenarios")
}

fn scenario_path(scenario: &str) -> PathBuf {
    scenarios_dir().join(format!("{scenario}.json"))
}

struct KillOnDrop(Child);

impl Drop for KillOnDrop {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}

fn wait_until(mut condition: impl FnMut() -> bool, timeout: Duration) {
    let deadline = Instant::now() + timeout;
    while !condition() {
        assert!(Instant::now() < deadline, "timed out waiting for condition");
        thread::sleep(Duration::from_millis(20));
    }
}

fn progress_count(lines: &[String]) -> usize {
    lines
        .iter()
        .filter(|line| {
            serde_json::from_str::<Value>(line).is_ok_and(|value| {
                value.get("method").and_then(Value::as_str) == Some(PROGRESS_METHOD)
            })
        })
        .count()
}

#[test]
fn scenario_files_exercise_every_failure_mode() {
    let mut observed = Vec::new();
    for entry in fs::read_dir(scenarios_dir()).expect("read scenarios") {
        let path = entry.expect("scenario entry").path();
        if path.extension().and_then(|ext| ext.to_str()) != Some("json") {
            continue;
        }
        let path = path.to_string_lossy();
        let scenario = Scenario::from_file(&path).expect("parse scenario");
        observed.extend(scenario.tools.into_iter().map(|tool| tool.failure_mode));
        if scenario.deny_checkpoints {
            observed.push(FailureMode::DenyCheckpoint);
        }
    }

    for mode in [
        FailureMode::None,
        FailureMode::Crash,
        FailureMode::Hang,
        FailureMode::BadJson,
        FailureMode::SlowProgress,
        FailureMode::DenyCheckpoint,
        FailureMode::Oom,
    ] {
        assert!(observed.contains(&mode), "no scenario for {mode:?}");
    }
}

#[test]
fn healthy_tool_streams_progress_and_result() {
    let mut app = MockProcess::start("healthy");
    app.call("mock.inspect", 1);
    let lines = app.lines_until_response(1);

    assert_eq!(progress_count(&lines), 3);
    let response: Value =
        serde_json::from_str(lines.last().expect("response line")).expect("parse response");
    assert_eq!(response["result"]["status"], "ok");
}

#[test]
fn bad_json_mode_emits_unparseable_response() {
    let mut app = MockProcess::start("bad_json");
    app.call("mock.inspect", 1);
    let malformed = app.next_line();
    assert!(malformed.contains(PROGRESS_METHOD));

    let malformed = app.next_line();
    assert!(malformed.contains("\"result\":"));
    assert!(serde_json::from_str::<Value>(&malformed).is_err());
}

#[test]
fn crash_mode_exits_after_progress() {
    let mut app = MockProcess::start("crash");
    app.call("mock.inspect", 1);
    let mut progress_seen = false;
    for _ in 0..8 {
        let mut line = String::new();
        if app.stdout.read_line(&mut line).expect("read line") == 0 {
            break;
        }
        if line.contains(PROGRESS_METHOD) {
            progress_seen = true;
        }
    }

    assert!(progress_seen);
    assert_eq!(app.wait_for_exit(), 70);
}

#[test]
fn oom_mode_exits_with_simulated_kill_code() {
    let mut app = MockProcess::start("oom");
    app.call("mock.inspect", 1);
    let mut progress_seen = false;
    for _ in 0..8 {
        let mut line = String::new();
        if app.stdout.read_line(&mut line).expect("read line") == 0 {
            break;
        }
        if line.contains(PROGRESS_METHOD) {
            progress_seen = true;
        }
    }

    assert!(progress_seen);
    assert_eq!(app.wait_for_exit(), 137);
}

#[test]
fn hang_mode_keeps_process_alive_after_progress() {
    let mut app = MockProcess::start("hang");
    app.call("mock.inspect", 1);
    assert!(app.next_line().contains(PROGRESS_METHOD));

    thread::sleep(Duration::from_millis(200));
    assert!(app.child.try_wait().expect("poll child").is_none());
}

#[test]
fn deny_checkpoint_mode_returns_json_rpc_error() {
    let mut app = MockProcess::start("deny_checkpoint");
    app.call("checkpoint.create", 7);
    let lines = app.lines_until_response(7);
    let response: Value =
        serde_json::from_str(lines.last().expect("response line")).expect("parse response");

    assert_eq!(response["error"]["code"], -32001);
    assert!(
        response["error"]["message"]
            .as_str()
            .is_some_and(|m| m.contains("denied"))
    );
}

#[test]
fn socket_mode_publishes_manifest_and_serves_tools() {
    let runtime = tempfile::tempdir().expect("temp runtime dir");
    let child = Command::new(env!("CARGO_BIN_EXE_starship-mock-app"))
        .arg("--scenario")
        .arg(scenario_path("healthy"))
        .env("XDG_RUNTIME_DIR", runtime.path())
        .env("XDG_DATA_HOME", runtime.path())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .expect("spawn socket server");
    let _server = KillOnDrop(child);

    let socket = runtime.path().join("starship/bridges/mock.sock");
    wait_until(|| socket.exists(), Duration::from_secs(5));
    assert!(runtime.path().join("starship/plugins/mock.json").exists());

    let mut stream = UnixStream::connect(&socket).expect("connect to socket");
    writeln!(
        stream,
        "{{\"jsonrpc\":\"2.0\",\"id\":1,\"method\":\"tools/call\",\"params\":{{\"name\":\"mock.inspect\",\"arguments\":{{}}}}}}"
    )
    .expect("write request");
    stream.flush().expect("flush request");

    let mut reader = BufReader::new(stream);
    let mut response = String::new();
    loop {
        response.clear();
        assert_ne!(reader.read_line(&mut response).expect("read response"), 0);
        let value: Value = serde_json::from_str(response.trim()).expect("parse response");
        if value.get("id").and_then(Value::as_u64) == Some(1) {
            assert_eq!(value["result"]["status"], "ok");
            break;
        }
    }
}

#[test]
fn slow_progress_mode_spreads_notifications_over_time() {
    let mut app = MockProcess::start("slow_progress");
    let started = Instant::now();
    app.call("mock.inspect", 1);
    let lines = app.lines_until_response(1);

    assert_eq!(progress_count(&lines), 4);
    assert!(started.elapsed() >= Duration::from_millis(80));
}
