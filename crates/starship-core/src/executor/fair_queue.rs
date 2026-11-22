use serde::{Deserialize, Serialize};
use std::cmp::Ordering;
use std::collections::{BinaryHeap, HashSet};

/// Priority tier for inter-mission scheduling (GAP-CLOSURE-PLAN.md §15.5).
/// User-initiated interactive requests preempt background cron/scheduled jobs.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MissionPriority {
    /// Foreground, interactive user trigger from Cockpit or Floating HUD (highest priority).
    UserInitiated = 1,
    /// Automated background tasks or scheduled maintenance jobs (lower priority).
    Scheduled = 2,
}

impl MissionPriority {
    /// Numeric rank where lower value indicates higher scheduling precedence.
    #[must_use]
    pub fn rank(&self) -> u8 {
        match self {
            Self::UserInitiated => 1,
            Self::Scheduled => 2,
        }
    }
}

/// A queued mission awaiting execution resources.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct QueuedMission {
    pub mission_id: i64,
    pub title: String,
    pub priority: MissionPriority,
    pub submitted_at_ms: u64,
}

impl Ord for QueuedMission {
    fn cmp(&self, other: &Self) -> Ordering {
        // Rust's BinaryHeap is a max-heap.
        // We want:
        // 1. Lower priority.rank() to come FIRST (UserInitiated > Scheduled)
        // 2. Lower submitted_at_ms to come FIRST (FIFO for same priority)
        other
            .priority
            .rank()
            .cmp(&self.priority.rank())
            .then_with(|| other.submitted_at_ms.cmp(&self.submitted_at_ms))
            .then_with(|| other.mission_id.cmp(&self.mission_id))
    }
}

impl PartialOrd for QueuedMission {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

/// Outcome of attempting to dispatch a mission into the execution queue.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MissionDispatchDecision {
    /// Slot available; mission starts immediately.
    StartImmediately,
    /// Concurrency capacity saturated; mission queued awaiting active completion.
    Queued {
        queue_position: usize,
        reason: String,
    },
}

/// Fair queueing mission scheduler managing bounded concurrent mission execution
/// (GAP-CLOSURE-PLAN.md §15.5).
#[derive(Debug, Clone)]
pub struct FairMissionQueue {
    max_concurrent_running: usize,
    running_mission_ids: HashSet<i64>,
    waiting_heap: BinaryHeap<QueuedMission>,
}

impl Default for FairMissionQueue {
    fn default() -> Self {
        Self {
            max_concurrent_running: 1, // Default: 1 active mission at a time
            running_mission_ids: HashSet::new(),
            waiting_heap: BinaryHeap::new(),
        }
    }
}

impl FairMissionQueue {
    #[must_use]
    pub fn new(max_concurrent_running: usize) -> Self {
        Self {
            max_concurrent_running: if max_concurrent_running == 0 {
                1
            } else {
                max_concurrent_running
            },
            running_mission_ids: HashSet::new(),
            waiting_heap: BinaryHeap::new(),
        }
    }

    /// Sets the maximum concurrent active missions allowed to run in parallel.
    pub fn set_max_concurrent(&mut self, limit: usize) {
        self.max_concurrent_running = if limit == 0 { 1 } else { limit };
    }

    #[must_use]
    pub fn max_concurrent(&self) -> usize {
        self.max_concurrent_running
    }

    #[must_use]
    pub fn running_count(&self) -> usize {
        self.running_mission_ids.len()
    }

    #[must_use]
    pub fn queued_count(&self) -> usize {
        self.waiting_heap.len()
    }

    #[must_use]
    pub fn is_running(&self, mission_id: i64) -> bool {
        self.running_mission_ids.contains(&mission_id)
    }

    /// Submits a mission for scheduling.
    /// If concurrency capacity permits, the mission starts immediately.
    /// Otherwise, it enters the priority queue.
    pub fn submit(
        &mut self,
        mission_id: i64,
        title: impl Into<String>,
        priority: MissionPriority,
        now_ms: u64,
    ) -> MissionDispatchDecision {
        if self.running_mission_ids.len() < self.max_concurrent_running {
            self.running_mission_ids.insert(mission_id);
            MissionDispatchDecision::StartImmediately
        } else {
            let item = QueuedMission {
                mission_id,
                title: title.into(),
                priority,
                submitted_at_ms: now_ms,
            };
            self.waiting_heap.push(item);
            MissionDispatchDecision::Queued {
                queue_position: self.waiting_heap.len(),
                reason: format!(
                    "Mission limit reached ({} running). Waiting in queue (priority: {:?})",
                    self.max_concurrent_running, priority
                ),
            }
        }
    }

    /// Signals that a running mission has completed (succeeded, failed, or aborted).
    /// Releases the execution slot and returns the next highest priority mission to start, if any.
    pub fn complete_and_pop_next(&mut self, completed_mission_id: i64) -> Option<QueuedMission> {
        self.running_mission_ids.remove(&completed_mission_id);

        if self.running_mission_ids.len() < self.max_concurrent_running
            && let Some(next) = self.waiting_heap.pop()
        {
            self.running_mission_ids.insert(next.mission_id);
            return Some(next);
        }
        None
    }

    /// Cancels a queued or running mission.
    pub fn cancel(&mut self, mission_id: i64) -> bool {
        if self.running_mission_ids.remove(&mission_id) {
            return true;
        }

        // Filter out from binary heap
        let existing: Vec<QueuedMission> = self.waiting_heap.drain().collect();
        let mut found = false;
        for item in existing {
            if item.mission_id == mission_id {
                found = true;
            } else {
                self.waiting_heap.push(item);
            }
        }
        found
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_two_missions_queue_and_run_in_order() {
        let mut queue = FairMissionQueue::new(1); // Max 1 active mission
        assert_eq!(queue.max_concurrent(), 1);

        // Mission 1 (User-initiated) submitted at t=100 -> starts immediately
        let d1 = queue.submit(
            1,
            "Edit Tutorial Video",
            MissionPriority::UserInitiated,
            100,
        );
        assert_eq!(d1, MissionDispatchDecision::StartImmediately);
        assert_eq!(queue.running_count(), 1);
        assert_eq!(queue.queued_count(), 0);

        // Mission 2 (User-initiated) submitted at t=200 -> queues at pos 1
        let d2 = queue.submit(2, "Clean Audio Track", MissionPriority::UserInitiated, 200);
        assert!(matches!(
            d2,
            MissionDispatchDecision::Queued {
                queue_position: 1,
                ..
            }
        ));
        assert_eq!(queue.running_count(), 1);
        assert_eq!(queue.queued_count(), 1);

        // Mission 1 completes -> Mission 2 is automatically popped and promoted to running
        let next = queue.complete_and_pop_next(1);
        assert!(next.is_some());
        let next_mission = next.unwrap();
        assert_eq!(next_mission.mission_id, 2);
        assert_eq!(queue.running_count(), 1);
        assert_eq!(queue.queued_count(), 0);

        // Mission 2 completes -> No remaining queued missions
        let finish = queue.complete_and_pop_next(2);
        assert!(finish.is_none());
        assert_eq!(queue.running_count(), 0);
        assert_eq!(queue.queued_count(), 0);
    }

    #[test]
    fn test_user_initiated_preempts_scheduled_in_waiting_queue() {
        let mut queue = FairMissionQueue::new(1);

        // Mission 1 (active) starts
        queue.submit(1, "Active User Task", MissionPriority::UserInitiated, 100);

        // Mission 2: Scheduled background task queued at t=150
        queue.submit(2, "Nightly Cache Cleanup", MissionPriority::Scheduled, 150);

        // Mission 3: User clicks "Record Now" at t=200
        queue.submit(
            3,
            "Urgent User Recording",
            MissionPriority::UserInitiated,
            200,
        );

        // Mission 4: Another Scheduled task queued at t=250
        queue.submit(4, "Routine Health Audit", MissionPriority::Scheduled, 250);

        assert_eq!(queue.queued_count(), 3);

        // When Mission 1 finishes, Mission 3 (UserInitiated) MUST run first, despite Mission 2 arriving earlier!
        let next1 = queue.complete_and_pop_next(1).expect("popped first");
        assert_eq!(
            next1.mission_id, 3,
            "UserInitiated task must preempt Scheduled tasks"
        );

        // When Mission 3 finishes, Mission 2 (Scheduled, FIFO t=150) runs next
        let next2 = queue.complete_and_pop_next(3).expect("popped second");
        assert_eq!(next2.mission_id, 2, "Earlier scheduled task runs next");

        // When Mission 2 finishes, Mission 4 (Scheduled, FIFO t=250) runs last
        let next3 = queue.complete_and_pop_next(2).expect("popped third");
        assert_eq!(next3.mission_id, 4);

        // Queue is drained
        assert!(queue.complete_and_pop_next(4).is_none());
    }
}
