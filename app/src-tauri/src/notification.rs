use ashpd::desktop::notification::{Notification, NotificationProxy, Priority};
use std::collections::HashMap;
use std::sync::{Arc, Mutex, PoisonError};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};
use tracing::{info, warn};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NotificationTrigger {
    ApprovalNeeded,
    MissionFinished,
    MissionFailed,
    MissionStalled,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SentNotificationRecord {
    pub trigger: NotificationTrigger,
    pub mission_id: i64,
    pub step_id: Option<i64>,
    pub title: String,
    pub body: String,
    pub sent_at_epoch_ms: u64,
}

#[derive(Debug, Default)]
struct TrackerState {
    last_global_sent: Option<Instant>,
    sent_keys: HashMap<(NotificationTrigger, i64, Option<i64>), Instant>,
    history: Vec<SentNotificationRecord>,
}

#[derive(Debug, Clone, Default)]
pub struct NotificationManager {
    tracker: Arc<Mutex<TrackerState>>,
}

impl NotificationManager {
    #[must_use]
    pub fn new() -> Self {
        Self {
            tracker: Arc::new(Mutex::new(TrackerState::default())),
        }
    }

    /// Check if a notification is permitted according to rate limiting and deduplication rules.
    ///
    /// Rules (`GAP-CLOSURE-PLAN.md` §13.7):
    /// - `ApprovalNeeded`: once per (`mission_id`, `step_id`), cooldown 60s.
    /// - `MissionFinished`: exactly once per `mission_id` (cooldown 1 day / persistent).
    /// - `MissionFailed`: exactly once per `mission_id`.
    /// - `MissionStalled`: allowed only if `stall_seconds >= 120` (2 min), cooldown 300s (5 min).
    /// - Global rate limit: at most 1 notification every 1000ms.
    #[must_use]
    pub fn can_send(
        &self,
        trigger: NotificationTrigger,
        mission_id: i64,
        step_id: Option<i64>,
        stall_seconds: Option<u64>,
    ) -> bool {
        let tracker = self.tracker.lock().unwrap_or_else(PoisonError::into_inner);
        let now = Instant::now();

        // Stalled condition: must be stalled for at least 120 seconds
        if trigger == NotificationTrigger::MissionStalled {
            let stalled_for = stall_seconds.unwrap_or(0);
            if stalled_for < 120 {
                return false;
            }
        }

        // Global cooldown of 1 second between any desktop notification
        if let Some(last_global) = tracker.last_global_sent
            && now.duration_since(last_global) < Duration::from_millis(1000)
        {
            return false;
        }

        // Per-key deduplication and cooldown
        let key = (trigger, mission_id, step_id);
        if let Some(last_sent) = tracker.sent_keys.get(&key) {
            let elapsed = now.duration_since(*last_sent);
            match trigger {
                NotificationTrigger::MissionFinished | NotificationTrigger::MissionFailed => {
                    // Exactly once per mission lifetime
                    return false;
                }
                NotificationTrigger::ApprovalNeeded => {
                    // Cooldown 60s
                    if elapsed < Duration::from_secs(60) {
                        return false;
                    }
                }
                NotificationTrigger::MissionStalled => {
                    // Cooldown 300s (5 min)
                    if elapsed < Duration::from_secs(300) {
                        return false;
                    }
                }
            }
        }

        true
    }

    /// Dispatch a desktop notification via ashpd portal with rate limiting and deduplication.
    #[allow(clippy::too_many_arguments)]
    pub async fn dispatch(
        &self,
        trigger: NotificationTrigger,
        mission_id: i64,
        step_id: Option<i64>,
        stall_seconds: Option<u64>,
        title: &str,
        body: &str,
        priority: Priority,
    ) -> bool {
        let should_send = self.can_send(trigger, mission_id, step_id, stall_seconds);
        if !should_send {
            info!(
                target: "starship_notification",
                trigger = ?trigger,
                mission_id,
                step_id,
                "Desktop notification deduplicated or rate-limited"
            );
            return false;
        }

        let now_epoch_ms = u64::try_from(
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_millis(),
        )
        .unwrap_or(u64::MAX);

        // Record in tracker
        {
            let mut tracker = self.tracker.lock().unwrap_or_else(PoisonError::into_inner);
            let key = (trigger, mission_id, step_id);
            let now = Instant::now();
            tracker.last_global_sent = Some(now);
            tracker.sent_keys.insert(key, now);
            tracker.history.push(SentNotificationRecord {
                trigger,
                mission_id,
                step_id,
                title: title.to_string(),
                body: body.to_string(),
                sent_at_epoch_ms: now_epoch_ms,
            });
        }

        info!(
            target: "starship_notification",
            trigger = ?trigger,
            mission_id,
            step_id,
            title,
            "Dispatching desktop portal notification"
        );

        // Attempt portal delivery asynchronously with tight timeout to prevent blocking if portal daemon is unresponsive
        let portal_result = tokio::time::timeout(Duration::from_millis(150), async {
            let proxy = NotificationProxy::new().await?;
            let notification = Notification::new(title)
                .body(Some(body))
                .priority(Some(priority));
            let notif_id = format!("starship-{mission_id}-{}", step_id.unwrap_or(0));
            proxy.add_notification(&notif_id, notification).await?;
            Ok::<(), ashpd::Error>(())
        })
        .await;

        match portal_result {
            Ok(Err(e)) => {
                warn!(
                    target: "starship_notification",
                    "Portal notification delivery could not connect to desktop daemon: {e}"
                );
            }
            Err(_) => {
                warn!(
                    target: "starship_notification",
                    "Portal notification timed out waiting for desktop daemon (daemon unavailable or headless)"
                );
            }
            Ok(Ok(())) => {}
        }

        true
    }

    #[must_use]
    pub fn sent_count(&self) -> usize {
        let tracker = self.tracker.lock().unwrap_or_else(PoisonError::into_inner);
        tracker.history.len()
    }

    #[must_use]
    pub fn history(&self) -> Vec<SentNotificationRecord> {
        let tracker = self.tracker.lock().unwrap_or_else(PoisonError::into_inner);
        tracker.history.clone()
    }

    pub fn clear(&self) {
        let mut tracker = self.tracker.lock().unwrap_or_else(PoisonError::into_inner);
        tracker.last_global_sent = None;
        tracker.sent_keys.clear();
        tracker.history.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_completing_mock_mission_raises_exactly_one_notification() {
        let notifier = NotificationManager::new();
        let mission_id = 42;

        // First mission completion raises notification
        let sent_1 = notifier
            .dispatch(
                NotificationTrigger::MissionFinished,
                mission_id,
                None,
                None,
                "Mission Completed",
                "Mission #42 finished successfully.",
                Priority::Normal,
            )
            .await;
        assert!(sent_1, "First completion must raise notification");

        // Second duplicate completion must be deduped
        let sent_2 = notifier
            .dispatch(
                NotificationTrigger::MissionFinished,
                mission_id,
                None,
                None,
                "Mission Completed",
                "Mission #42 finished successfully.",
                Priority::Normal,
            )
            .await;
        assert!(!sent_2, "Duplicate completion must be deduplicated");

        // Third duplicate completion must also be deduped
        let sent_3 = notifier
            .dispatch(
                NotificationTrigger::MissionFinished,
                mission_id,
                None,
                None,
                "Mission Completed",
                "Mission #42 finished successfully.",
                Priority::Normal,
            )
            .await;
        assert!(!sent_3, "Subsequent completion must be deduplicated");

        assert_eq!(
            notifier.sent_count(),
            1,
            "Completing a mock mission raises exactly one notification"
        );
        let history = notifier.history();
        assert_eq!(history[0].trigger, NotificationTrigger::MissionFinished);
        assert_eq!(history[0].mission_id, 42);
    }

    #[tokio::test]
    async fn test_stalled_notification_requires_120_seconds_and_dedupes() {
        let notifier = NotificationManager::new();
        let mission_id = 100;
        let step_id = Some(3);

        // Stall duration 60s (< 120s) must NOT raise notification
        let sent_short = notifier
            .dispatch(
                NotificationTrigger::MissionStalled,
                mission_id,
                step_id,
                Some(60),
                "Mission Stalled",
                "Step stalled for 60s",
                Priority::High,
            )
            .await;
        assert!(!sent_short, "Stall < 120s must not notify");

        // Stall duration 125s (>= 120s) raises notification
        let sent_valid = notifier
            .dispatch(
                NotificationTrigger::MissionStalled,
                mission_id,
                step_id,
                Some(125),
                "Mission Stalled",
                "Step stalled for 125s",
                Priority::High,
            )
            .await;
        assert!(sent_valid, "Stall >= 120s must notify");

        // Immediate subsequent stall event is deduped by cooldown
        let sent_dup = notifier
            .dispatch(
                NotificationTrigger::MissionStalled,
                mission_id,
                step_id,
                Some(130),
                "Mission Stalled",
                "Step stalled for 130s",
                Priority::High,
            )
            .await;
        assert!(!sent_dup, "Duplicate stall within cooldown must be deduped");

        assert_eq!(notifier.sent_count(), 1);
    }

    #[tokio::test]
    async fn test_approval_needed_and_failure_deduplication() {
        let notifier = NotificationManager::new();
        let mission_id = 200;

        // Approval needed for step 1
        let app_1 = notifier
            .dispatch(
                NotificationTrigger::ApprovalNeeded,
                mission_id,
                Some(1),
                None,
                "Approval Required",
                "Tier 2 action in EEP",
                Priority::High,
            )
            .await;
        assert!(app_1);

        // Immediate retry for step 1 is deduped
        let app_1_dup = notifier
            .dispatch(
                NotificationTrigger::ApprovalNeeded,
                mission_id,
                Some(1),
                None,
                "Approval Required",
                "Tier 2 action in EEP",
                Priority::High,
            )
            .await;
        assert!(!app_1_dup);

        // Mission failed raises notification
        // Note: simulate a brief delay so global rate limit doesn't block it
        tokio::time::sleep(Duration::from_millis(1100)).await;

        let fail_1 = notifier
            .dispatch(
                NotificationTrigger::MissionFailed,
                mission_id,
                None,
                None,
                "Mission Failed",
                "Step execution failed",
                Priority::Urgent,
            )
            .await;
        assert!(fail_1);

        // Duplicate failure is deduped
        let fail_dup = notifier
            .dispatch(
                NotificationTrigger::MissionFailed,
                mission_id,
                None,
                None,
                "Mission Failed",
                "Step execution failed",
                Priority::Urgent,
            )
            .await;
        assert!(!fail_dup);
    }
}
