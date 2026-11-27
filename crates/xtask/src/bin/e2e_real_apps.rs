use serde::Serialize;
use starship_core::pipeline::{
    DiagnosticsPipeline, ScreencastAssemblyRequest, ScreencastPipeline, SystemMetricsReport,
    pipeline_record_and_edit,
};
use std::fs;
use std::io::{BufRead, BufReader, Write};
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::mpsc;
use std::time::Duration;

const HELLO: &str = r#"{"jsonrpc":"2.0","id":1,"method":"starship/hello"}"#;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
enum Support {
    StdioMcp,
    SocketBridgeNonMcp,
    NoMcp,
    Missing,
}

impl Support {
    fn is_real(self) -> bool {
        self == Self::StdioMcp
    }

    fn status(self) -> &'static str {
        if self.is_real() { "REAL" } else { "MOCK-ONLY" }
    }
}

#[derive(Debug, Serialize)]
struct AppProbe {
    app_id: String,
    name: String,
    binary: String,
    support: Support,
    status: String,
    hello_response: String,
    stderr_excerpt: String,
}

#[derive(Debug, Serialize)]
struct StepResult {
    app_id: String,
    tool_name: String,
    risk_tier: u8,
    status: String,
}

#[derive(Debug, Serialize)]
struct PipelineResult {
    name: String,
    plan_title: String,
    steps: Vec<StepResult>,
}

#[derive(Debug, Serialize)]
struct Report {
    generated_at: String,
    workspace_version: &'static str,
    apps: Vec<AppProbe>,
    pipelines: Vec<PipelineResult>,
    real_step_execution: Option<serde_json::Value>,
    blockers: Vec<String>,
}

fn find_binary(explicit: &str) -> Option<PathBuf> {
    let path = Path::new(explicit);
    if path.is_file() && fs::metadata(path).is_ok_and(|m| m.permissions().mode() & 0o111 != 0) {
        return Some(path.to_path_buf());
    }
    let name = path.file_name()?.to_str()?;
    let path_var = std::env::var_os("PATH")?;
    std::env::split_paths(&path_var)
        .map(|dir| dir.join(name))
        .find(|candidate| {
            candidate.is_file()
                && fs::metadata(candidate).is_ok_and(|m| m.permissions().mode() & 0o111 != 0)
        })
}

fn read_line_with_timeout<R: BufRead + Send + 'static>(
    reader: R,
    timeout: Duration,
) -> Option<String> {
    let (sender, receiver) = mpsc::channel();
    std::thread::spawn(move || {
        let mut reader = reader;
        let mut line = String::new();
        let ok = reader.read_line(&mut line).is_ok_and(|n| n > 0);
        let _ = sender.send(if ok {
            line.trim().to_owned()
        } else {
            String::new()
        });
    });
    receiver.recv_timeout(timeout).ok()
}

fn probe_app(app_id: &str, name: &str, explicit: &str) -> AppProbe {
    let Some(binary) = find_binary(explicit) else {
        return AppProbe {
            app_id: app_id.to_owned(),
            name: name.to_owned(),
            binary: explicit.to_owned(),
            support: Support::Missing,
            status: Support::Missing.status().to_owned(),
            hello_response: String::new(),
            stderr_excerpt: "binary not found on PATH".to_owned(),
        };
    };

    let Ok(mut child) = Command::new(&binary)
        .arg("--mcp")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
    else {
        return AppProbe {
            app_id: app_id.to_owned(),
            name: name.to_owned(),
            binary: binary.display().to_string(),
            support: Support::NoMcp,
            status: Support::NoMcp.status().to_owned(),
            hello_response: String::new(),
            stderr_excerpt: "failed to spawn --mcp".to_owned(),
        };
    };

    if let Some(stdin) = child.stdin.as_mut() {
        let _ = writeln!(stdin, "{HELLO}");
        let _ = stdin.flush();
    }
    let stdout = child.stdout.take().expect("stdout pipe");
    let hello =
        read_line_with_timeout(BufReader::new(stdout), Duration::from_secs(2)).unwrap_or_default();
    let _ = child.kill();
    let stderr_bytes = child
        .stderr
        .take()
        .and_then(|mut pipe| {
            use std::io::Read;
            let mut bytes = Vec::new();
            pipe.read_to_end(&mut bytes).ok().map(|_| bytes)
        })
        .unwrap_or_default();
    let stderr_excerpt = String::from_utf8_lossy(&stderr_bytes)
        .lines()
        .filter(|line| !line.trim().is_empty())
        .take(3)
        .collect::<Vec<_>>()
        .join(" | ");

    let support = if hello.contains("\"appId\"") && hello.contains("\"tools\"") {
        Support::StdioMcp
    } else if stderr_excerpt.contains("MCP bridge listening") {
        Support::SocketBridgeNonMcp
    } else {
        Support::NoMcp
    };

    AppProbe {
        app_id: app_id.to_owned(),
        name: name.to_owned(),
        binary: binary.display().to_string(),
        support,
        status: support.status().to_owned(),
        hello_response: hello,
        stderr_excerpt,
    }
}

fn call_real_step(binary: &Path) -> Option<serde_json::Value> {
    let mut child = Command::new(binary)
        .arg("--mcp")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .ok()?;
    let request = r#"{"jsonrpc":"2.0","id":2,"method":"tools/call","params":{"name":"tuneup.get_health","arguments":{}}}"#;
    if let Some(stdin) = child.stdin.as_mut() {
        let _ = writeln!(stdin, "{request}");
        let _ = stdin.flush();
    }
    let stdout = child.stdout.take()?;
    let response = read_line_with_timeout(BufReader::new(stdout), Duration::from_secs(3))?;
    let _ = child.kill();
    serde_json::from_str(&response).ok()
}

fn screencast_plan() -> serde_json::Value {
    let telemetry = starship_core::pipeline::CetTelemetry::from_jsonl_str(
        r#"{"type":"header","version":"cet/1.0","recording_fps":60.0,"duration_ms":90000,"audio_sample_rate":48000}
{"type":"silence","start_ms":1000,"end_ms":4200,"duration_ms":3200}"#,
    )
    .expect("parse CET fixture");
    let result = ScreencastPipeline::plan_assembly(
        &ScreencastAssemblyRequest {
            recording_path: "/tmp/e2e/demo.mkv".to_owned(),
            cet_path: None,
            clean_audio: true,
            audio_preset: Some("voice_isolate".to_owned()),
            intro_script: Some("Welcome to the demo".to_owned()),
            target_timeline_track: 1,
        },
        Some(&telemetry),
    )
    .expect("build screencast plan");
    serde_json::to_value(&result.task_sheet).expect("serialize plan")
}

fn diagnostics_plan() -> serde_json::Value {
    let plan = DiagnosticsPipeline::plan_system_maintenance(&SystemMetricsReport {
        live_connections_count: 14,
        total_bandwidth_kbps: 480.5,
        disk_used_gb: 128.4,
        disk_free_gb: 38.6,
        cache_bloat_mb: 4300.0,
        reclaimable_items: vec![
            "~/.cache/cargo/registry".to_owned(),
            "~/.cache/npm".to_owned(),
        ],
    });
    serde_json::to_value(&plan).expect("serialize plan")
}

fn pipeline_from_plan(name: &str, plan: &serde_json::Value, apps: &[AppProbe]) -> PipelineResult {
    let steps = plan
        .get("steps")
        .and_then(|steps| steps.as_array())
        .map_or_default(|steps| {
            steps
                .iter()
                .map(|step| {
                    let app_id = step
                        .get("app_id")
                        .or_else(|| step.get("appId"))
                        .and_then(|v| v.as_str())
                        .unwrap_or("?");
                    let tool_name = step
                        .get("tool_name")
                        .or_else(|| step.get("toolName"))
                        .and_then(|v| v.as_str())
                        .unwrap_or("?");
                    let app = apps.iter().find(|app| app.app_id == app_id);
                    let support = app.map_or(Support::Missing, |app| app.support);
                    let tool_supported = app.is_some_and(|app| {
                        app.hello_response
                            .contains(&format!("\"name\":\"{tool_name}\""))
                    });
                    let status = if support.is_real() && tool_supported {
                        "REAL"
                    } else {
                        "MOCK-ONLY"
                    };
                    StepResult {
                        app_id: app_id.to_owned(),
                        tool_name: tool_name.to_owned(),
                        risk_tier: u8::try_from(
                            step.get("risk_tier")
                                .or_else(|| step.get("riskTier"))
                                .and_then(serde_json::Value::as_u64)
                                .unwrap_or(0),
                        )
                        .unwrap_or(0),
                        status: status.to_owned(),
                    }
                })
                .collect()
        });
    PipelineResult {
        name: name.to_owned(),
        plan_title: plan
            .get("title")
            .or_else(|| plan.get("goal"))
            .and_then(|v| v.as_str())
            .unwrap_or("?")
            .to_owned(),
        steps,
    }
}

fn write_text(path: &Path, contents: &str) {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).expect("create evidence dir");
    }
    fs::write(path, contents).unwrap_or_else(|error| panic!("write {}: {error}", path.display()));
}

fn main() {
    let out = Path::new("docs/evidence/e2e");
    fs::create_dir_all(out).expect("create e2e evidence dir");

    let targets = [
        ("ssp", "Screen Studio Pro", "/usr/bin/screenstudio-pro"),
        ("eep", "Easy Edits Pro", "/usr/bin/easy-edits-pro"),
        ("alp", "Audio Labs Pro", "/usr/bin/audio-labs-pro"),
        ("nwp", "Net Watch Pro", "/usr/bin/net-watch-pro"),
        ("atp", "Agentic Tuneup Pro", "/usr/bin/agentic-tuneup-pro"),
    ];
    let eep_home = format!(
        "{}/.local/bin/easy-edits-pro",
        std::env::var("HOME").unwrap_or_default()
    );
    let apps: Vec<AppProbe> = targets
        .iter()
        .map(|(id, name, binary)| {
            let explicit = if *id == "eep" {
                eep_home.as_str()
            } else {
                binary
            };
            probe_app(id, name, explicit)
        })
        .collect();

    let plans = [
        ("pipeline_screencast", screencast_plan()),
        (
            "pipeline_record_and_edit",
            serde_json::to_value(pipeline_record_and_edit("record a demo", 60))
                .expect("serialize plan"),
        ),
        ("pipeline_diagnostics", diagnostics_plan()),
    ];
    for (name, plan) in &plans {
        write_text(
            &out.join(format!("{name}-plan.json")),
            &serde_json::to_string_pretty(plan).expect("pretty plan"),
        );
        write_text(
            &out.join(format!("{name}.log")),
            &format!(
                "{name}: generated {} steps via no-LLM planner\n",
                plan.get("steps")
                    .and_then(|v| v.as_array())
                    .map_or(0, Vec::len)
            ),
        );
    }

    let pipelines = plans
        .iter()
        .map(|(name, plan)| pipeline_from_plan(name, plan, &apps))
        .collect();

    let atp_binary = apps
        .iter()
        .find(|app| app.app_id == "atp" && app.support.is_real())
        .map(|app| PathBuf::from(&app.binary));
    let real_step_execution = atp_binary
        .as_deref()
        .and_then(call_real_step)
        .map(|response| {
            serde_json::json!({
                "app": "atp",
                "tool": "tuneup.get_health",
                "response": response,
            })
        });

    let blockers: Vec<String> = apps
        .iter()
        .filter(|app| !app.support.is_real())
        .map(|app| {
            format!(
                "{}: {} (binary: {})",
                app.app_id,
                match app.support {
                    Support::Missing => "not installed",
                    Support::NoMcp => "no working --mcp stdio mode",
                    Support::SocketBridgeNonMcp => "custom socket bridge, not MCP-compatible",
                    Support::StdioMcp => unreachable!(),
                },
                app.binary
            )
        })
        .collect();

    let report = Report {
        generated_at: "2026-11-25".to_owned(),
        workspace_version: env!("CARGO_PKG_VERSION"),
        apps,
        pipelines,
        real_step_execution,
        blockers,
    };
    let json = serde_json::to_string_pretty(&report).expect("serialize report");
    write_text(&out.join("report.json"), &json);
    write_text(&out.join("report.md"), &render_markdown(&report));
    write_text(&out.join("report.html"), &render_html(&report));
    println!("E2E report written to {}", out.display());
}

fn render_markdown(report: &Report) -> String {
    let mut lines = vec![
        "# Real-app E2E matrix".to_owned(),
        String::new(),
        format!("Generated: {}", report.generated_at),
        String::new(),
        "| App | Binary | MCP support | Status |".to_owned(),
        "| :-- | :-- | :-- | :-- |".to_owned(),
    ];
    for app in &report.apps {
        lines.push(format!(
            "| {} | {} | {:?} | {} |",
            app.name, app.binary, app.support, app.status
        ));
    }
    lines.push(String::new());
    lines.push("## Pipeline steps".to_owned());
    for pipeline in &report.pipelines {
        lines.push(format!("### {}", pipeline.name));
        for step in &pipeline.steps {
            lines.push(format!(
                "- `{}` / `{}` (Tier {}) — {}",
                step.app_id, step.tool_name, step.risk_tier, step.status
            ));
        }
    }
    lines.push(String::new());
    lines.push("## Blockers".to_owned());
    for blocker in &report.blockers {
        lines.push(format!("- {blocker}"));
    }
    lines.join("\n")
}

fn render_html(report: &Report) -> String {
    let rows = report
        .apps
        .iter()
        .map(|app| {
            format!(
                "<tr><td>{}</td><td class=\"mono\">{}</td><td>{:?}</td><td><span class=\"badge {}\">{}</span></td></tr>",
                app.name,
                app.binary,
                app.support,
                if app.support.is_real() { "ok" } else { "warn" },
                app.status
            )
        })
        .collect::<Vec<_>>()
        .join("\n");
    let blockers = report
        .blockers
        .iter()
        .map(|blocker| format!("<li>{blocker}</li>"))
        .collect::<Vec<_>>()
        .join("\n");
    format!(
        "<!doctype html><html><head><meta charset=\"utf-8\"><title>Real-app E2E matrix</title><style>
        body{{background:#050505;color:#e6e6e6;font-family:Inter,system-ui,sans-serif;margin:0;padding:32px}}
        h1{{font-size:24px;font-weight:500;margin:0 0 8px}} h2{{font-size:15px;font-weight:500;margin:28px 0 8px}}
        p{{color:#9a9a9a;font-size:13px}} table{{border-collapse:collapse;width:100%;font-size:12px}}
        td,th{{border-bottom:1px solid #26272a;padding:8px;text-align:left}} th{{color:#8a8a8a;font-weight:500}}
        .mono{{font-family:monospace;color:#b8b8b8}} .badge{{padding:2px 8px;border-radius:4px;font-size:11px}}
        .ok{{background:#12301f;color:#4cc38a}} .warn{{background:#332511;color:#e5a94b}} li{{font-size:12px;margin:4px 0}}
        </style></head><body><h1>Real-app E2E matrix</h1><p>Generated {} · version {}</p>
        <table><tr><th>App</th><th>Binary</th><th>MCP support</th><th>Status</th></tr>{}</table>
        <h2>Blockers</h2><ul>{}</ul></body></html>",
        report.generated_at, report.workspace_version, rows, blockers
    )
}
