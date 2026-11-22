use crate::PluginTool;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Mutex;

pub const TOOL_CHECKPOINT_CREATE: &str = "checkpoint.create";
pub const TOOL_CHECKPOINT_RESTORE: &str = "checkpoint.restore";
pub const TOOL_CHECKPOINT_DROP: &str = "checkpoint.drop";

/// Standard payload returned by `checkpoint.create`.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct CheckpointCreateResult {
    pub checkpoint_id: String,
    pub label: String,
}

/// Standard payload returned by `checkpoint.restore`.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct CheckpointRestoreResult {
    pub checkpoint_id: String,
    pub restored: bool,
}

/// Trait implemented by sub-applications exposing mutating or destructive (Tier 2+) operations (GAP-CLOSURE-PLAN.md §11.7).
pub trait CheckpointHandler: Send + Sync {
    /// Creates a snapshot/checkpoint of the application state before mutation.
    fn create_checkpoint(&self, label: &str) -> Result<String, String>;
    /// Restores the application state to the specified checkpoint on failure.
    fn restore_checkpoint(&self, checkpoint_id: &str) -> Result<bool, String>;
    /// Discards a checkpoint once the mission step or plan succeeds.
    fn drop_checkpoint(&self, checkpoint_id: &str) -> Result<bool, String>;
}

/// Generates standard MCP tool definitions for the Checkpoint Contract (v1.1).
#[must_use]
pub fn standard_checkpoint_tools() -> Vec<PluginTool> {
    vec![
        PluginTool {
            name: TOOL_CHECKPOINT_CREATE.to_string(),
            description: "Creates an undo snapshot/checkpoint of the active project or resource prior to mutation".to_string(),
            risk_tier: 1, // Safe inspection/snapshot
            parameters: serde_json::json!({
                "type": "object",
                "properties": {
                    "label": { "type": "string", "description": "Human-readable label for this checkpoint" }
                },
                "required": ["label"]
            }),
            resource_profile: Some(crate::resource::ResourceProfile::light(128)),
        },
        PluginTool {
            name: TOOL_CHECKPOINT_RESTORE.to_string(),
            description: "Restores project state to a prior snapshot ID in the event of step failure".to_string(),
            risk_tier: 2, // Mutating rollback
            parameters: serde_json::json!({
                "type": "object",
                "properties": {
                    "checkpointId": { "type": "string", "description": "ID of the checkpoint to restore" }
                },
                "required": ["checkpointId"]
            }),
            resource_profile: Some(crate::resource::ResourceProfile::medium(256)),
        },
        PluginTool {
            name: TOOL_CHECKPOINT_DROP.to_string(),
            description: "Drops and cleans up an expired or committed checkpoint".to_string(),
            risk_tier: 1, // Safe cleanup
            parameters: serde_json::json!({
                "type": "object",
                "properties": {
                    "checkpointId": { "type": "string", "description": "ID of the checkpoint to drop" }
                },
                "required": ["checkpointId"]
            }),
            resource_profile: Some(crate::resource::ResourceProfile::light(64)),
        },
    ]
}

/// Validates that if an application plugin declares mutating/destructive tools (Tier >= 2),
/// it strictly satisfies the Checkpoint Contract by providing `checkpoint.create`, `checkpoint.restore`, and `checkpoint.drop`.
///
/// # Errors
/// Returns an error message identifying the missing checkpoint tools if the contract is violated.
pub fn validate_checkpoint_contract(tools: &[PluginTool]) -> Result<(), String> {
    let has_tier_2_plus = tools.iter().any(|t| t.risk_tier >= 2);
    if !has_tier_2_plus {
        return Ok(());
    }

    let mut has_create = false;
    let mut has_restore = false;
    let mut has_drop = false;

    for tool in tools {
        match tool.name.as_str() {
            TOOL_CHECKPOINT_CREATE => has_create = true,
            TOOL_CHECKPOINT_RESTORE => has_restore = true,
            TOOL_CHECKPOINT_DROP => has_drop = true,
            _ => {}
        }
    }

    if !has_create || !has_restore || !has_drop {
        return Err(format!(
            "App declares Tier 2+ mutating tools but violates Checkpoint Contract v1.1: missing [create: {has_create}, restore: {has_restore}, drop: {has_drop}]"
        ));
    }

    Ok(())
}

/// In-memory mock checkpoint store for sub-app testing and verification.
#[derive(Debug, Default)]
pub struct MockCheckpointStore {
    checkpoints: Mutex<HashMap<String, String>>,
    current_data: Mutex<String>,
}

impl MockCheckpointStore {
    #[must_use]
    pub fn new(initial_data: impl Into<String>) -> Self {
        Self {
            checkpoints: Mutex::new(HashMap::new()),
            current_data: Mutex::new(initial_data.into()),
        }
    }

    #[must_use]
    pub fn get_current_data(&self) -> String {
        self.current_data.lock().map_or_default(|d| d.clone())
    }

    pub fn mutate_data(&self, new_data: impl Into<String>) {
        if let Ok(mut lock) = self.current_data.lock() {
            *lock = new_data.into();
        }
    }
}

impl CheckpointHandler for MockCheckpointStore {
    fn create_checkpoint(&self, label: &str) -> Result<String, String> {
        let mut ckpts = self
            .checkpoints
            .lock()
            .map_err(|_| "Lock poisoned".to_string())?;
        let id = format!("ckpt_{label}_{}", ckpts.len() + 1);
        let current = self.get_current_data();
        ckpts.insert(id.clone(), current);
        Ok(id)
    }

    fn restore_checkpoint(&self, checkpoint_id: &str) -> Result<bool, String> {
        let ckpts = self
            .checkpoints
            .lock()
            .map_err(|_| "Lock poisoned".to_string())?;
        if let Some(saved) = ckpts.get(checkpoint_id) {
            let mut curr = self
                .current_data
                .lock()
                .map_err(|_| "Lock poisoned".to_string())?;
            (*curr).clone_from(saved);
            Ok(true)
        } else {
            Err(format!("Checkpoint '{checkpoint_id}' not found"))
        }
    }

    fn drop_checkpoint(&self, checkpoint_id: &str) -> Result<bool, String> {
        let mut ckpts = self
            .checkpoints
            .lock()
            .map_err(|_| "Lock poisoned".to_string())?;
        Ok(ckpts.remove(checkpoint_id).is_some())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_checkpoint_contract_validation() {
        // Safe-only tools don't require checkpoints
        let safe_tools = vec![PluginTool {
            name: "telemetry.query".into(),
            description: "Read only".into(),
            risk_tier: 1,
            parameters: serde_json::json!({}),
            resource_profile: None,
        }];
        assert!(validate_checkpoint_contract(&safe_tools).is_ok());

        // Tier 2 tool without checkpoint contract fails
        let mutating_tools_incomplete = vec![PluginTool {
            name: "timeline.cut".into(),
            description: "Mutating cut".into(),
            risk_tier: 2,
            parameters: serde_json::json!({}),
            resource_profile: None,
        }];
        assert!(validate_checkpoint_contract(&mutating_tools_incomplete).is_err());

        // Tier 2 tool with complete standard checkpoint tools passes
        let mut mutating_tools_complete = mutating_tools_incomplete;
        mutating_tools_complete.extend(standard_checkpoint_tools());
        assert!(validate_checkpoint_contract(&mutating_tools_complete).is_ok());
    }

    #[test]
    fn test_mock_checkpoint_create_mutate_restore_lifecycle() {
        // Initial state before mutation
        let initial_timeline = "Track 1: 00:00 - 05:00 [Original High-Quality Audio]";
        let store = MockCheckpointStore::new(initial_timeline);
        assert_eq!(store.get_current_data(), initial_timeline);

        // 1. Host creates checkpoint before first Tier 2 step
        let ckpt_id = store
            .create_checkpoint("pre_cut")
            .expect("create checkpoint");
        assert!(ckpt_id.starts_with("ckpt_pre_cut_"));

        // 2. Step executes and mutates project state
        store.mutate_data("Track 1: [CORRUPTED / ACCIDENTAL TRUNCATION]");
        assert_ne!(store.get_current_data(), initial_timeline);

        // 3. Step fails -> Host triggers checkpoint.restore
        let restored = store
            .restore_checkpoint(&ckpt_id)
            .expect("restore checkpoint");
        assert!(restored);

        // Verify state is byte-identical to pre-mutation snapshot
        assert_eq!(store.get_current_data(), initial_timeline);

        // 4. Host drops checkpoint
        let dropped = store.drop_checkpoint(&ckpt_id).expect("drop checkpoint");
        assert!(dropped);

        // Subsequent restore attempt fails
        assert!(store.restore_checkpoint(&ckpt_id).is_err());
    }
}
