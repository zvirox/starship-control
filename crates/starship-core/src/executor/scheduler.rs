use crate::{CoreError, ErrorKind};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet, VecDeque};

/// Execution node in the DAG scheduler.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct DagNode {
    pub step_id: i64,
    pub dependencies: Vec<i64>,
    pub timeout_ms: u64,
}

/// DAG Scheduler managing dependency resolution, ready sets, and concurrency limits (GAP-CLOSURE-PLAN.md §11.3).
#[derive(Debug, Clone)]
pub struct DagScheduler {
    nodes: HashMap<i64, DagNode>,
    concurrency_limit: usize,
    default_timeout_ms: u64,
}

impl DagScheduler {
    pub const DEFAULT_CONCURRENCY: usize = 2;
    pub const MIN_TIMEOUT_MS: u64 = 10_000; // 10 seconds floor

    /// Creates a new DAG scheduler from a list of nodes, performing cycle detection at load.
    ///
    /// # Errors
    /// Returns `ipc.cycle_detected` if the dependency graph contains cycles.
    pub fn new(nodes: Vec<DagNode>, concurrency_limit: usize) -> Result<Self, CoreError> {
        let mut node_map = HashMap::new();
        for node in nodes {
            node_map.insert(node.step_id, node);
        }

        let scheduler = Self {
            nodes: node_map,
            concurrency_limit: if concurrency_limit == 0 {
                Self::DEFAULT_CONCURRENCY
            } else {
                concurrency_limit
            },
            default_timeout_ms: Self::MIN_TIMEOUT_MS,
        };

        scheduler.detect_cycles()?;
        Ok(scheduler)
    }

    /// Dynamically adjusts the bounded parallelism limit (e.g. governor overload adjustment).
    pub fn set_concurrency_limit(&mut self, limit: usize) {
        self.concurrency_limit = if limit == 0 { 1 } else { limit };
    }

    #[must_use]
    pub fn concurrency_limit(&self) -> usize {
        self.concurrency_limit
    }

    /// Verifies the DAG is acyclic using Kahn's topological sorting algorithm.
    fn detect_cycles(&self) -> Result<(), CoreError> {
        let mut in_degree: HashMap<i64, usize> = HashMap::new();
        let mut adj: HashMap<i64, Vec<i64>> = HashMap::new();

        for &id in self.nodes.keys() {
            in_degree.entry(id).or_insert(0);
            adj.entry(id).or_default();
        }

        for node in self.nodes.values() {
            for &dep in &node.dependencies {
                // dep -> node.step_id
                adj.entry(dep).or_default().push(node.step_id);
                *in_degree.entry(node.step_id).or_insert(0) += 1;
            }
        }

        let mut queue: VecDeque<i64> = VecDeque::new();
        for (&id, &deg) in &in_degree {
            if deg == 0 {
                queue.push_back(id);
            }
        }

        let mut visited_count = 0;
        while let Some(u) = queue.pop_front() {
            visited_count += 1;
            if let Some(neighbors) = adj.get(&u) {
                for &v in neighbors {
                    if let Some(deg) = in_degree.get_mut(&v) {
                        *deg -= 1;
                        if *deg == 0 {
                            queue.push_back(v);
                        }
                    }
                }
            }
        }

        if visited_count != self.nodes.len() {
            return Err(CoreError::new(
                ErrorKind::Ipc,
                "ipc.cycle_detected",
                false,
                "Cyclic dependency detected in execution plan DAG",
                Some(serde_json::json!({
                    "totalNodes": self.nodes.len(),
                    "resolvedNodes": visited_count,
                })),
            ));
        }

        Ok(())
    }

    /// Computes the set of steps ready for immediate dispatch.
    /// A step is ready if all its declared dependencies are satisfied (in `completed_step_ids`)
    /// and it has not already been started or completed.
    ///
    /// Respects the bounded parallelism limit (`concurrency_limit - running_step_ids.len()`).
    #[must_use]
    pub fn compute_ready_set(
        &self,
        completed_step_ids: &HashSet<i64>,
        running_step_ids: &HashSet<i64>,
    ) -> Vec<i64> {
        let available_slots = self
            .concurrency_limit
            .saturating_sub(running_step_ids.len());
        if available_slots == 0 {
            return Vec::new();
        }

        let mut ready = Vec::new();
        for (id, node) in &self.nodes {
            if completed_step_ids.contains(id) || running_step_ids.contains(id) {
                continue;
            }

            // Check if all dependencies are satisfied
            let deps_satisfied = node
                .dependencies
                .iter()
                .all(|dep| completed_step_ids.contains(dep));

            if deps_satisfied {
                ready.push(*id);
            }
        }

        // Sort for deterministic execution ordering
        ready.sort_unstable();
        ready.truncate(available_slots);
        ready
    }

    /// Checks if a running step has exceeded its configured or minimum timeout.
    #[must_use]
    pub fn is_step_timed_out(&self, step_id: i64, elapsed_ms: u64) -> bool {
        let timeout = self
            .nodes
            .get(&step_id)
            .map_or(self.default_timeout_ms, |n| {
                n.timeout_ms.max(Self::MIN_TIMEOUT_MS)
            });
        elapsed_ms > timeout
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cycle_detection_in_dag() {
        // Cycle: 1 -> 2 -> 3 -> 1
        let cyclic_nodes = vec![
            DagNode {
                step_id: 1,
                dependencies: vec![3],
                timeout_ms: 10_000,
            },
            DagNode {
                step_id: 2,
                dependencies: vec![1],
                timeout_ms: 10_000,
            },
            DagNode {
                step_id: 3,
                dependencies: vec![2],
                timeout_ms: 10_000,
            },
        ];

        let res = DagScheduler::new(cyclic_nodes, 2);
        assert!(res.is_err());
        assert_eq!(res.unwrap_err().code, "ipc.cycle_detected");
    }

    #[test]
    fn test_diamond_dag_execution_order_and_bounded_parallelism() {
        // Diamond DAG:
        //      1 (Root)
        //     / \
        //    2   3 (Branches)
        //     \ /
        //      4 (Join)
        let nodes = vec![
            DagNode {
                step_id: 1,
                dependencies: vec![],
                timeout_ms: 15_000,
            },
            DagNode {
                step_id: 2,
                dependencies: vec![1],
                timeout_ms: 15_000,
            },
            DagNode {
                step_id: 3,
                dependencies: vec![1],
                timeout_ms: 15_000,
            },
            DagNode {
                step_id: 4,
                dependencies: vec![2, 3],
                timeout_ms: 15_000,
            },
        ];

        let scheduler = DagScheduler::new(nodes, 2).expect("scheduler created");
        let mut completed = HashSet::new();
        let mut running = HashSet::new();

        // 1. Initial ready set: only step 1 is ready
        let ready = scheduler.compute_ready_set(&completed, &running);
        assert_eq!(ready, vec![1]);

        // Step 1 starts running
        running.insert(1);
        assert_eq!(
            scheduler.compute_ready_set(&completed, &running),
            Vec::<i64>::new()
        );

        // Step 1 completes
        running.remove(&1);
        completed.insert(1);

        // 2. Both step 2 and step 3 become ready concurrently (bounded by parallelism = 2)
        let ready = scheduler.compute_ready_set(&completed, &running);
        assert_eq!(ready, vec![2, 3]);

        // Dispatch step 2 and step 3
        running.insert(2);
        running.insert(3);
        assert_eq!(
            scheduler.compute_ready_set(&completed, &running),
            Vec::<i64>::new()
        );

        // Step 2 completes, step 3 is still running
        running.remove(&2);
        completed.insert(2);

        // Step 4 must NOT be ready yet because step 3 is not complete
        let ready = scheduler.compute_ready_set(&completed, &running);
        assert_eq!(
            ready,
            Vec::<i64>::new(),
            "Step 4 should not be ready until both 2 and 3 complete"
        );

        // Step 3 completes
        running.remove(&3);
        completed.insert(3);

        // 3. Step 4 is now ready
        let ready = scheduler.compute_ready_set(&completed, &running);
        assert_eq!(ready, vec![4]);

        // Step 4 completes
        completed.insert(4);
        assert_eq!(
            scheduler.compute_ready_set(&completed, &running),
            Vec::<i64>::new()
        );
    }

    #[test]
    fn test_step_timeout_detection() {
        let nodes = vec![
            DagNode {
                step_id: 1,
                dependencies: vec![],
                timeout_ms: 12_000,
            },
            DagNode {
                step_id: 2,
                dependencies: vec![],
                timeout_ms: 5_000,
            }, // clamped to MIN 10_000
        ];
        let scheduler = DagScheduler::new(nodes, 2).expect("scheduler");

        // Node 1: timeout 12_000
        assert!(!scheduler.is_step_timed_out(1, 10_000));
        assert!(!scheduler.is_step_timed_out(1, 12_000));
        assert!(scheduler.is_step_timed_out(1, 12_001));

        // Node 2: clamped to 10_000 floor
        assert!(!scheduler.is_step_timed_out(2, 9_999));
        assert!(!scheduler.is_step_timed_out(2, 10_000));
        assert!(scheduler.is_step_timed_out(2, 10_001));
    }
}
