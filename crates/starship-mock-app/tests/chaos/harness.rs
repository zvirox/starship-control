use starship_core::executor::{
    LeaseSupervisor, MissionEvent, MissionState, PauseReason, StepEvent, StepState,
    transition_mission, transition_step,
};
use starship_core::planner::{TaskSheet, TaskSheetStep};
use starship_core::safety::SafetyArbiter;
use starship_db::{Database, NewAuditEntry, NewMissionEvent};
use std::path::{Path, PathBuf};
use std::thread;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

pub fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("clock is after epoch")
        .as_millis()
        .try_into()
        .expect("timestamp fits u64")
}

fn now_ms_i64() -> i64 {
    i64::try_from(now_ms()).unwrap_or(i64::MAX)
}

pub fn scenario_path(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("scenarios")
        .join(format!("{name}.json"))
}

fn sample_sheet() -> TaskSheet {
    TaskSheet::new(
        "chaos goal",
        "Chaos mission",
        "Runs one mutating step under fault injection",
        vec![TaskSheetStep {
            step_index: 0,
            app_id: "eep".to_owned(),
            tool_name: "eep.timeline.cut_interval".to_owned(),
            arguments: serde_json::json!({ "startMs": 1000, "endMs": 4200 }),
            risk_tier: 2,
            diff_summary: "Remove dead air from the timeline".to_owned(),
            depends_on: Vec::new(),
        }],
    )
}

pub struct ChaosMission {
    pub _temp: tempfile::TempDir,
    pub db: Database,
    pub mission_id: i64,
    pub plan_id: i64,
    pub step_id: i64,
    pub arbiter: SafetyArbiter,
}

impl ChaosMission {
    pub fn new() -> Self {
        let temp = tempfile::tempdir().expect("temp dir");
        let db = Database::open_path(temp.path().join("library.db")).expect("open db");
        let mission_id = db
            .create_mission(
                "chaos-mission",
                "Chaos mission",
                "Fault injection verification",
                "qwen2.5-coder-7b-q4",
                "local_gguf",
            )
            .expect("create mission");
        let plan = db
            .create_plan(
                mission_id,
                "Chaos plan",
                "One mutating step",
                &sample_sheet(),
            )
            .expect("create plan");
        let step_id = db
            .get_active_plan(mission_id)
            .expect("get plan")
            .expect("active plan")
            .1[0]
            .id;

        let mission = Self {
            _temp: temp,
            db,
            mission_id,
            plan_id: plan.id,
            step_id,
            arbiter: SafetyArbiter::new("chaos-test-salt"),
        };
        mission.audit("chaos.setup", "success", None);
        mission.mission_event(MissionEvent::PlanGenerated);
        mission.mission_event(MissionEvent::Start);
        mission
    }

    pub fn audit(&self, action: &str, status: &str, code: Option<&str>) {
        self.db
            .record_audit_log(&NewAuditEntry {
                mission_id: Some(self.mission_id),
                app_id: "chaos".to_owned(),
                action_type: action.to_owned(),
                risk_tier: 2,
                payload_json: serde_json::json!({ "code": code }).to_string(),
                status: status.to_owned(),
                signature_token: None,
            })
            .expect("record audit");
    }

    pub fn mission_event(&self, event: MissionEvent) {
        self.db
            .record_mission_event(&NewMissionEvent {
                mission_id: self.mission_id,
                step_id: None,
                ts_ms: now_ms_i64(),
                kind: "mission_transition".to_owned(),
                payload_json: serde_json::to_string(&event).expect("serialize mission event"),
            })
            .expect("record mission event");
    }

    pub fn step_event(&self, event: StepEvent) {
        self.db
            .record_mission_event(&NewMissionEvent {
                mission_id: self.mission_id,
                step_id: Some(self.step_id),
                ts_ms: now_ms_i64(),
                kind: "step_transition".to_owned(),
                payload_json: serde_json::to_string(&event).expect("serialize step event"),
            })
            .expect("record step event");
    }

    pub fn start_step(&self) {
        transition_step(StepState::Pending, StepEvent::Schedule).expect("schedule step");
        self.step_event(StepEvent::Schedule);
        transition_step(StepState::Ready, StepEvent::ApproveAndDispatch).expect("dispatch step");
        self.db
            .update_step_execution(self.step_id, "running", None, None)
            .expect("mark step running");
        self.step_event(StepEvent::ApproveAndDispatch);
        self.audit("chaos.tool_call", "running", None);
    }

    pub fn fail_step(&self, code: &str) {
        transition_step(StepState::Running, StepEvent::FailFinal).expect("final step failure");
        self.db
            .update_step_execution(self.step_id, "failed", None, Some(code))
            .expect("mark step failed");
        self.step_event(StepEvent::FailFinal);
        self.audit("chaos.tool_failure", "failed", Some(code));
        self.set_mission_status("failed");
        self.mission_event(MissionEvent::Fail);
    }

    pub fn interrupt_step(&self) {
        transition_step(StepState::Running, StepEvent::CrashInterrupt).expect("interrupt step");
        self.step_event(StepEvent::CrashInterrupt);
        self.audit("chaos.core_interrupt", "interrupted", Some("chaos.kill9"));
    }

    pub fn pause(&self, reason: PauseReason) {
        transition_mission(MissionState::Running, MissionEvent::Pause(reason))
            .expect("pause mission");
        self.mission_event(MissionEvent::Pause(reason));
        self.set_mission_status("paused");
    }

    pub fn set_mission_status(&self, status: &str) {
        self.db
            .with_conn(|conn| {
                conn.execute(
                    "UPDATE missions SET status = ?1 WHERE id = ?2",
                    rusqlite::params![status, self.mission_id],
                )?;
                Ok(())
            })
            .expect("update mission status");
    }

    pub fn assert_final(&self, expected: MissionState, code: Option<&str>) {
        let (state, _) = self
            .db
            .rebuild_mission_state(self.mission_id)
            .expect("rebuild state");
        assert_eq!(state, expected, "unexpected final mission state");

        let audited = self.db.verify_audit_chain().expect("audit chain");
        let minimum = if code.is_some() { 3 } else { 2 };
        assert!(
            audited >= minimum,
            "expected at least {minimum} audit entries"
        );
        let logs = self.db.list_audit_logs(100, 0).expect("audit logs");
        assert!(
            logs.iter()
                .any(|entry| entry.mission_id == Some(self.mission_id))
        );
        if let Some(code) = code {
            assert!(
                logs.iter().any(|entry| entry.payload_json.contains(code)),
                "audit log missing chaos code {code}"
            );
        }

        let leases = LeaseSupervisor::new().clean_expired_leases(now_ms());
        assert!(leases.is_empty(), "chaos run leaked leases: {leases:?}");
        assert!(
            self.db
                .with_conn(
                    |conn| conn.query_row("PRAGMA busy_timeout", [], |row| row.get::<_, i64>(0))
                )
                .is_ok(),
            "database connection left unusable"
        );
    }
}

fn is_zombie(pid: u32) -> bool {
    std::fs::read_to_string(format!("/proc/{pid}/stat"))
        .ok()
        .and_then(|stat| {
            stat.rsplit_once(')')
                .map(|(_, rest)| rest.trim_start().starts_with('Z'))
        })
        .unwrap_or(false)
}

pub fn assert_process_gone(pid: u32) {
    let deadline = std::time::Instant::now() + Duration::from_secs(3);
    while Path::new(&format!("/proc/{pid}")).exists()
        && !is_zombie(pid)
        && std::time::Instant::now() < deadline
    {
        thread::sleep(Duration::from_millis(20));
    }
    let gone = !Path::new(&format!("/proc/{pid}")).exists() || is_zombie(pid);
    assert!(gone, "orphan process {pid}");
}
