use super::harness::ChaosMission;
use starship_core::MissionState;
use starship_core::executor::{FailureClass, Governor, classify_error};
use starship_health::HealthState;
use std::io::Write;
use std::time::Duration;

const ENOSPC: i32 = 28;

#[test]
fn disk_full_maps_to_resource_failure_and_pauses_governor() {
    let chaos = ChaosMission::new();
    chaos.start_step();

    let mut device = std::fs::OpenOptions::new()
        .write(true)
        .open("/dev/full")
        .expect("open /dev/full");
    let error = device
        .write_all(b"chaos")
        .expect_err("/dev/full returns ENOSPC");
    assert_eq!(error.raw_os_error(), Some(ENOSPC));

    let code = "disk.no_space";
    assert_eq!(classify_error(code), FailureClass::Resource);

    let mut governor = Governor::new(2, Duration::from_millis(50));
    let directive = governor.on_health_update(HealthState::Critical);
    assert!(
        directive.should_pause,
        "critical health must pause execution"
    );
    assert!(!directive.allow_heavy_steps);

    chaos.fail_step(code);
    chaos.assert_final(MissionState::Failed, Some(code));
}
