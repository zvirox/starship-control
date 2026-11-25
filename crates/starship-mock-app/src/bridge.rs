use crate::faults::{FaultRuntime, TerminalFault};
use crate::scenario::{FailureMode, Scenario, ToolScenario};
use starship_plugin::{
    AppManifest, PluginBridge, PluginTool, ProgressReporter, ResourceProfile, socket_path_for,
};
use std::io::{BufRead, Write};
use std::thread;
use std::time::Duration;

fn build_manifest(scenario: &Scenario) -> AppManifest {
    let binary_path =
        std::env::current_exe().map_or_default(|path| path.to_string_lossy().into_owned());

    AppManifest {
        schema_version: "1.0".to_owned(),
        app_id: scenario.app_id.clone(),
        name: scenario.name.clone(),
        version: scenario.version.clone(),
        binary_path,
        launch_args: Vec::new(),
        mcp_cli_args: vec!["--mcp".to_owned()],
        socket_path: Some(
            socket_path_for(&scenario.app_id)
                .to_string_lossy()
                .into_owned(),
        ),
        capabilities: serde_json::json!({
            "scriptedTools": scenario.tools.len(),
            "failureModes": scenario.tools.iter().map(|tool| tool.failure_mode).collect::<Vec<_>>(),
        }),
    }
}

fn plugin_tool(tool: &ToolScenario) -> PluginTool {
    PluginTool {
        name: tool.name.clone(),
        description: tool.description.clone(),
        risk_tier: tool.risk_tier,
        parameters: serde_json::json!({
            "type": "object",
            "properties": {},
            "additionalProperties": true,
        }),
        resource_profile: Some(ResourceProfile::light(64)),
    }
}

fn tool_handler(
    tool: ToolScenario,
) -> impl Fn(serde_json::Value, &ProgressReporter) -> Result<serde_json::Value, String> {
    move |_arguments, _progress| {
        if tool.latency_ms > 0 {
            thread::sleep(Duration::from_millis(tool.latency_ms));
        }
        if tool.failure_mode == FailureMode::DenyCheckpoint {
            return Err("mock app: checkpoint denied by scenario".to_owned());
        }
        Ok(tool.result.clone())
    }
}

/// Builds a plugin bridge whose tools and failure modes come from the scenario.
#[must_use]
pub fn build_bridge(scenario: &Scenario) -> PluginBridge {
    let mut bridge = PluginBridge::new(build_manifest(scenario));
    let deny_checkpoints = scenario.deny_checkpoints;
    bridge.enable_checkpoints(
        move |label| {
            if deny_checkpoints {
                Err("mock app: checkpoint denied by scenario".to_owned())
            } else {
                Ok(format!("mock_ckpt_{label}"))
            }
        },
        |_id| Ok(()),
        |_id| Ok(()),
    );

    for tool in &scenario.tools {
        bridge.register_tool(plugin_tool(tool), tool_handler(tool.clone()));
    }
    bridge
}

fn terminal_fault(mode: FailureMode) -> Option<TerminalFault> {
    match mode {
        FailureMode::Crash => Some(TerminalFault::Crash),
        FailureMode::Hang => Some(TerminalFault::Hang),
        FailureMode::Oom => Some(TerminalFault::Oom),
        FailureMode::None
        | FailureMode::BadJson
        | FailureMode::SlowProgress
        | FailureMode::DenyCheckpoint => None,
    }
}

fn progress_token(request: &serde_json::Value) -> serde_json::Value {
    request
        .get("params")
        .and_then(|params| {
            params
                .get("_meta")
                .and_then(|meta| meta.get("progressToken"))
                .or_else(|| params.get("progressToken"))
        })
        .cloned()
        .unwrap_or_else(|| serde_json::Value::String("token_default".to_owned()))
}

fn emit_progress<W: Write>(
    writer: &mut W,
    token: &serde_json::Value,
    tool: &ToolScenario,
    scenario: &Scenario,
) -> std::io::Result<()> {
    let steps = tool.progress_steps.unwrap_or(scenario.progress_steps);
    let interval = if tool.failure_mode == FailureMode::SlowProgress {
        scenario.progress_interval_ms
    } else {
        0
    };
    let reporter = ProgressReporter::new(token.clone());
    for step in 1..=steps {
        let message = format!("{step}/{steps}");
        writer.write_all(
            reporter
                .format_notification(
                    f64::from(step) / f64::from(steps),
                    Some(1.0),
                    Some(&message),
                )
                .as_bytes(),
        )?;
        writer.flush()?;
        if interval > 0 && step < steps {
            thread::sleep(Duration::from_millis(interval));
        }
    }
    Ok(())
}

fn write_response<W: Write>(writer: &mut W, response: &str) -> std::io::Result<()> {
    writeln!(writer, "{response}")?;
    writer.flush()
}

fn write_error<W: Write>(
    writer: &mut W,
    id: &serde_json::Value,
    code: i64,
    message: &str,
) -> std::io::Result<()> {
    let response = serde_json::json!({
        "jsonrpc": "2.0",
        "id": id,
        "error": { "code": code, "message": message },
    });
    write_response(writer, &response.to_string())
}

fn write_malformed_json<W: Write>(writer: &mut W, id: &serde_json::Value) -> std::io::Result<()> {
    write_response(
        writer,
        &format!("{{\"jsonrpc\":\"2.0\",\"id\":{id},\"result\":"),
    )
}

fn dispatch_tool_call<W: Write>(
    writer: &mut W,
    bridge: &PluginBridge,
    scenario: &Scenario,
    faults: &dyn FaultRuntime,
    line: &str,
    request: &serde_json::Value,
    tool: &ToolScenario,
) -> std::io::Result<()> {
    let id = request
        .get("id")
        .cloned()
        .unwrap_or(serde_json::Value::Null);
    emit_progress(writer, &progress_token(request), tool, scenario)?;

    if let Some(fault) = terminal_fault(tool.failure_mode)
        && let Err(message) = faults.trigger(fault)
    {
        return write_error(writer, &id, -32000, &message);
    }

    match tool.failure_mode {
        FailureMode::BadJson => write_malformed_json(writer, &id),
        FailureMode::DenyCheckpoint => write_error(
            writer,
            &id,
            -32001,
            "mock app: checkpoint denied by scenario",
        ),
        _ => bridge
            .handle_request(line)
            .map_or(Ok(()), |response| write_response(writer, &response)),
    }
}

/// Serves line-delimited JSON-RPC with scenario-driven progress and fault injection.
///
/// # Errors
/// Returns an I/O error when the underlying stream cannot be read or written.
pub fn serve<R: BufRead, W: Write>(
    bridge: &PluginBridge,
    scenario: &Scenario,
    faults: &dyn FaultRuntime,
    mut reader: R,
    mut writer: W,
) -> std::io::Result<()> {
    let mut line = String::new();
    while reader.read_line(&mut line)? > 0 {
        let trimmed = line.trim();
        let request = serde_json::from_str::<serde_json::Value>(trimmed).ok();
        let tool_name = request
            .as_ref()
            .filter(|value| value.get("method").and_then(|m| m.as_str()) == Some("tools/call"))
            .and_then(|value| value.get("params"))
            .and_then(|params| params.get("name"))
            .and_then(|name| name.as_str());

        if let (Some(request), Some(name)) = (request.as_ref(), tool_name)
            && let Some(tool) = scenario.tool(name)
        {
            dispatch_tool_call(
                &mut writer,
                bridge,
                scenario,
                faults,
                trimmed,
                request,
                tool,
            )?;
        } else if let Some(response) = bridge.handle_request(trimmed) {
            write_response(&mut writer, &response)?;
        }
        line.clear();
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::faults::RecordingFaults;
    use std::io::Cursor;

    fn call_line(tool: &str) -> String {
        format!(
            "{{\"jsonrpc\":\"2.0\",\"id\":1,\"method\":\"tools/call\",\"params\":{{\"name\":\"{tool}\",\"arguments\":{{}},\"_meta\":{{\"progressToken\":\"p1\"}}}}}}\n"
        )
    }

    #[test]
    fn healthy_tool_returns_result_and_progress() {
        let scenario = Scenario::default();
        let bridge = build_bridge(&scenario);
        let faults = RecordingFaults::default();
        let mut output = Vec::new();
        let input = call_line("mock.inspect");

        serve(
            &bridge,
            &scenario,
            &faults,
            Cursor::new(input.as_bytes()),
            &mut output,
        )
        .unwrap();

        let output = String::from_utf8(output).unwrap();
        assert!(output.contains("\"progressToken\":\"p1\""));
        assert!(output.contains("\"status\":\"ok\""));
        assert_eq!(faults.triggered(), []);
    }

    #[test]
    fn fatal_modes_are_triggered_and_reported() {
        let mut scenario = Scenario::default();
        scenario.tools[0].failure_mode = FailureMode::Oom;
        let bridge = build_bridge(&scenario);
        let faults = RecordingFaults::default();
        let mut output = Vec::new();

        serve(
            &bridge,
            &scenario,
            &faults,
            Cursor::new(call_line("mock.inspect").as_bytes()),
            &mut output,
        )
        .unwrap();

        assert_eq!(faults.triggered(), vec![TerminalFault::Oom]);
        assert!(String::from_utf8(output).unwrap().contains("simulated Oom"));
    }

    #[test]
    fn bad_json_mode_emits_unparseable_response() {
        let mut scenario = Scenario::default();
        scenario.tools[0].failure_mode = FailureMode::BadJson;
        let bridge = build_bridge(&scenario);
        let faults = RecordingFaults::default();
        let mut output = Vec::new();

        serve(
            &bridge,
            &scenario,
            &faults,
            Cursor::new(call_line("mock.inspect").as_bytes()),
            &mut output,
        )
        .unwrap();

        let malformed = String::from_utf8(output)
            .unwrap()
            .lines()
            .any(|line| serde_json::from_str::<serde_json::Value>(line).is_err());
        assert!(malformed);
    }
}
