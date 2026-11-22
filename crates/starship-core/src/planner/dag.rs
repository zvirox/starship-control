use crate::CoreError;
use crate::planner::tasksheet::{TaskSheet, TaskSheetStep};
use std::collections::{HashMap, HashSet, VecDeque};

/// Directed Acyclic Graph (DAG) representing step dependencies and parallel execution stages.
#[derive(Debug, Clone)]
pub struct PlanDag {
    /// Mapping of `step_index` to the step definition.
    steps: HashMap<u32, TaskSheetStep>,
    /// Forward adjacency list: `step_index` -> list of steps that depend on it.
    dependents: HashMap<u32, Vec<u32>>,
    /// In-degree map: `step_index` -> number of unsatisfied dependencies.
    in_degrees: HashMap<u32, usize>,
    /// Calculated execution stages (each stage can run independently or in parallel).
    stages: Vec<Vec<u32>>,
}

impl PlanDag {
    /// Builds and validates a `PlanDag` from a `TaskSheet`.
    /// Performs cycle detection and computes topological execution stages.
    pub fn build(sheet: &TaskSheet) -> Result<Self, CoreError> {
        sheet.validate()?;

        let mut steps = HashMap::with_capacity(sheet.steps.len());
        let mut dependents: HashMap<u32, Vec<u32>> = HashMap::with_capacity(sheet.steps.len());
        let mut in_degrees: HashMap<u32, usize> = HashMap::with_capacity(sheet.steps.len());

        for step in &sheet.steps {
            steps.insert(step.step_index, step.clone());
            in_degrees.insert(step.step_index, step.depends_on.len());
            dependents.entry(step.step_index).or_default();
        }

        for step in &sheet.steps {
            for &dep in &step.depends_on {
                dependents.entry(dep).or_default().push(step.step_index);
            }
        }

        // Kahn's algorithm for topological sorting and stage generation
        let mut queue = VecDeque::new();
        let mut remaining_in_degrees = in_degrees.clone();

        for (&step_idx, &deg) in &remaining_in_degrees {
            if deg == 0 {
                queue.push_back(step_idx);
            }
        }

        let mut stages = Vec::new();
        let mut resolved_count = 0;

        while !queue.is_empty() {
            // All items currently in queue can execute in this stage
            let stage_size = queue.len();
            let mut current_stage = Vec::with_capacity(stage_size);

            for _ in 0..stage_size {
                if let Some(current_step) = queue.pop_front() {
                    current_stage.push(current_step);
                    resolved_count += 1;
                }
            }

            // Sort current stage by step_index for deterministic ordering
            current_stage.sort_unstable();

            // Decrease in-degrees of dependent steps
            for &step_idx in &current_stage {
                if let Some(next_steps) = dependents.get(&step_idx) {
                    for &next_idx in next_steps {
                        if let Some(deg) = remaining_in_degrees.get_mut(&next_idx) {
                            *deg -= 1;
                            if *deg == 0 {
                                queue.push_back(next_idx);
                            }
                        }
                    }
                }
            }

            stages.push(current_stage);
        }

        if resolved_count != sheet.steps.len() {
            return Err(CoreError::safety(
                "planner.cyclic_dependency",
                false,
                format!(
                    "Cyclic dependency detected in task sheet: resolved {} of {} steps",
                    resolved_count,
                    sheet.steps.len()
                ),
                None,
            ));
        }

        Ok(Self {
            steps,
            dependents,
            in_degrees,
            stages,
        })
    }

    /// Returns the number of initial dependencies for a given step.
    #[must_use]
    pub fn in_degree(&self, step_index: u32) -> usize {
        self.in_degrees.get(&step_index).copied().unwrap_or(0)
    }

    /// Returns the topologically sorted execution stages.
    #[must_use]
    pub fn stages(&self) -> &[Vec<u32>] {
        &self.stages
    }

    /// Retrieves a step by its `step_index`.
    #[must_use]
    pub fn get_step(&self, step_index: u32) -> Option<&TaskSheetStep> {
        self.steps.get(&step_index)
    }

    /// Returns true if all dependencies for a step are in the completed set.
    #[must_use]
    pub fn is_step_ready(&self, step_index: u32, completed_steps: &HashSet<u32>) -> bool {
        if let Some(step) = self.steps.get(&step_index) {
            step.depends_on
                .iter()
                .all(|dep| completed_steps.contains(dep))
        } else {
            false
        }
    }

    /// Returns all steps that depend on the given step index.
    #[must_use]
    pub fn get_dependents(&self, step_index: u32) -> &[u32] {
        self.dependents.get(&step_index).map_or(&[], Vec::as_slice)
    }

    /// Returns the total number of steps in this DAG.
    #[must_use]
    pub fn len(&self) -> usize {
        self.steps.len()
    }

    /// Returns true if the DAG contains no steps.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.steps.is_empty()
    }
}
