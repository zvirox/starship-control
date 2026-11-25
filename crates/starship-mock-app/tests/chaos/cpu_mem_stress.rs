use super::harness::{ChaosMission, now_ms};
use starship_core::MissionState;
use starship_core::executor::{
    AdmissionController, AdmissionDecision, CrashRecoveryManager, Governor, InterruptedStepInfo,
    PauseReason, RecoveryAction, StepState,
};
use starship_core::safety::RiskTier;
use starship_health::{
    CpuMetrics, HealthSample, HealthState, MemoryMetrics, OverloadDetector, OverloadThresholds,
};
use starship_plugin::ResourceProfile;
use std::time::Duration;

fn stressed_sample() -> HealthSample {
    HealthSample {
        timestamp_ms: now_ms(),
        cpu: Some(CpuMetrics {
            global_util_pct: 98.0,
            core_util_pct: vec![98.0, 97.0],
            load_avg_1m: 16.0,
            load_avg_5m: 15.0,
            load_avg_15m: 14.0,
            avg_freq_mhz: 3_200,
        }),
        memory: Some(MemoryMetrics {
            total_bytes: 16 * 1024 * 1024 * 1024,
            used_bytes: 15 * 1024 * 1024 * 1024,
            available_bytes: 128 * 1024 * 1024,
            available_pct: 2.0,
            swap_total_bytes: 0,
            swap_used_bytes: 0,
            swap_in_bytes_per_sec: 0.0,
        }),
        ..HealthSample::default()
    }
}

fn real_stress_available() -> bool {
    std::process::Command::new("stress-ng")
        .arg("--version")
        .output()
        .is_ok_and(|output| output.status.success())
}

#[test]
fn cpu_mem_stress_pauses_governor_and_queues_heavy_work() {
    let chaos = ChaosMission::new();
    chaos.start_step();

    if !real_stress_available() {
        eprintln!(
            "SKIP real stress-ng load: binary not installed; using synthesized health sample"
        );
    }
    let sample = stressed_sample();
    let mut detector = OverloadDetector::new(OverloadThresholds::default());
    let mut state = HealthState::Normal;
    for _ in 0..3 {
        state = detector.update(&sample).0;
    }
    assert_eq!(state, HealthState::Critical);

    let directive = Governor::new(2, Duration::from_millis(50)).on_health_update(state);
    assert!(directive.should_pause);
    assert!(!directive.allow_local_inference);

    let decision = AdmissionController::new().evaluate(&ResourceProfile::default(), Some(&sample));
    assert!(matches!(decision, AdmissionDecision::Queued { .. }));

    chaos.audit("chaos.governor_pause", "paused", Some("governor.paused"));
    chaos.pause(PauseReason::Governor);
    chaos.assert_final(
        MissionState::Paused(PauseReason::Governor),
        Some("governor.paused"),
    );

    let interrupted = InterruptedStepInfo {
        step_id: chaos.step_id,
        step_index: 0,
        app_id: "eep".to_owned(),
        tool_name: "eep.timeline.cut_interval".to_owned(),
        risk_tier: RiskTier::Mutating,
        checkpoint_id: None,
    };
    let (step_state, _) = CrashRecoveryManager::resolve_step_recovery(
        &interrupted,
        RecoveryAction::Abort,
        &chaos.arbiter,
        chaos.plan_id,
        now_ms(),
    )
    .expect("abort recovery");
    assert_eq!(step_state, StepState::Cancelled);
}
