//! `cargo xtask check-no-fakes` — Phase 17 rule 1: no fake data in release builds.
//!
//! Scans non-test source under `crates/` and `app/src/` for known simulation
//! markers, invented PIDs, mock chart data and hard-coded model ids. Test
//! modules (`#[cfg(test)]`, `/tests/`, `*.test.ts(x)`) and the model seed
//! migration are excluded; they are the legitimate places for fixtures.

use std::fs;
use std::path::{Path, PathBuf};

const LITERAL_PATTERNS: &[&str] = &[
    "Math.random",
    "simulate_tool_call",
    "mockSteps",
    "Default simulated",
    "Some(12345)",
];

const MODEL_ID: &str = "qwen2.5-coder-7b-q4";
const EXEMPTION: &str = "check-no-fakes: allowed";

pub fn run() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    // xtask lives at <workspace>/crates/xtask — go up two levels.
    let workspace = root
        .parent()
        .and_then(Path::parent)
        .expect("xtask lives in crates/")
        .to_path_buf();
    let mut files = Vec::new();
    collect_sources(&workspace.join("crates"), &mut files);
    collect_sources(&workspace.join("app/src"), &mut files);

    let mut violations: Vec<String> = Vec::new();
    for file in files {
        let text = match fs::read_to_string(&file) {
            Ok(text) => text,
            Err(e) => {
                eprintln!("check-no-fakes: cannot read {}: {e}", file.display());
                std::process::exit(2);
            }
        };
        let rel = file.strip_prefix(&workspace).unwrap_or(&file);
        let allowed_model_id = rel
            .components()
            .any(|c| matches!(c.as_os_str().to_str(), Some("migrations" | "tests")))
            || rel.to_string_lossy().ends_with(".test.ts");
        let mut in_test_module = false;
        for (line_no, line) in text.lines().enumerate() {
            let line_no = line_no + 1;
            if line.contains("#[cfg(test)]") {
                in_test_module = true;
            }
            if in_test_module || line.contains(EXEMPTION) {
                continue;
            }
            for pattern in LITERAL_PATTERNS {
                if line.contains(pattern) {
                    violations.push(format!("{}:{line_no}: {pattern}", rel.display()));
                }
            }
            if !allowed_model_id && line.contains(MODEL_ID) {
                violations.push(format!(
                    "{}:{line_no}: hard-coded model id {MODEL_ID}",
                    rel.display()
                ));
            }
            if is_numeric_telemetry_array(line) {
                violations.push(format!(
                    "{}:{line_no}: hard-coded numeric telemetry array",
                    rel.display()
                ));
            }
        }
    }

    if violations.is_empty() {
        println!("check-no-fakes: PASS — no simulation or invented telemetry in release source");
    } else {
        for violation in &violations {
            eprintln!("check-no-fakes: {violation}");
        }
        eprintln!("check-no-fakes: FAIL ({} violation(s))", violations.len());
        std::process::exit(1);
    }
}

fn collect_sources(dir: &Path, out: &mut Vec<PathBuf>) {
    let Ok(entries) = fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
        if path.is_dir() {
            if !matches!(
                name,
                "target" | "node_modules" | "tests" | "dist" | "evidence"
            ) {
                collect_sources(&path, out);
            }
        } else if matches!(
            path.extension().and_then(|e| e.to_str()),
            Some("rs" | "ts" | "tsx")
        ) && !name.ends_with(".test.ts")
            && !name.ends_with(".test.tsx")
        {
            out.push(path);
        }
    }
}

/// Four or more numeric literals in an array, excluding style/layout tuples.
fn is_numeric_telemetry_array(line: &str) -> bool {
    for excluded in ["borderRadius", "padding", "margin", "inset", "gridTemplate"] {
        if line.contains(excluded) {
            return false;
        }
    }
    let Some(open) = line.find('[') else {
        return false;
    };
    let Some(close) = line[open + 1..].find(']') else {
        return false;
    };
    let inner = &line[open + 1..open + 1 + close];
    let count = inner
        .split(',')
        .filter(|part| part.trim().parse::<f64>().is_ok())
        .count();
    count >= 4
}
