use crate::{CoreError, ErrorKind};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

/// Quarantined item metadata stored in the mission quarantine manifest (GAP-CLOSURE-PLAN.md §11.8).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct QuarantinedItem {
    pub original_path: String,
    pub quarantine_path: String,
    pub sha256: String,
    pub size_bytes: u64,
    pub quarantined_at_secs: u64,
}

/// Manifest stored in `$XDG_DATA_HOME/starship-control/quarantine/<mission>/manifest.json`.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct QuarantineManifest {
    pub mission_id: i64,
    pub created_at_secs: u64,
    pub ttl_days: u32,
    pub items: HashMap<String, QuarantinedItem>,
}

pub struct QuarantineManager {
    base_dir: PathBuf,
}

impl QuarantineManager {
    pub const DEFAULT_TTL_DAYS: u32 = 7;

    #[must_use]
    pub fn new(base_dir: impl Into<PathBuf>) -> Self {
        Self {
            base_dir: base_dir.into(),
        }
    }

    /// Default quarantine root directory: `$XDG_DATA_HOME/starship-control/quarantine/`.
    #[must_use]
    pub fn default_dir() -> PathBuf {
        dirs::data_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("starship-control")
            .join("quarantine")
    }

    fn mission_quarantine_dir(&self, mission_id: i64) -> PathBuf {
        self.base_dir.join(format!("mission_{mission_id}"))
    }

    fn manifest_path(&self, mission_id: i64) -> PathBuf {
        self.mission_quarantine_dir(mission_id)
            .join("manifest.json")
    }

    /// Loads or initializes the quarantine manifest for a mission.
    #[must_use]
    pub fn load_manifest(&self, mission_id: i64) -> QuarantineManifest {
        let path = self.manifest_path(mission_id);
        if let Ok(data) = fs::read_to_string(&path)
            && let Ok(manifest) = serde_json::from_str::<QuarantineManifest>(&data)
        {
            return manifest;
        }

        QuarantineManifest {
            mission_id,
            created_at_secs: 0,
            ttl_days: Self::DEFAULT_TTL_DAYS,
            items: HashMap::new(),
        }
    }

    fn save_manifest(&self, manifest: &QuarantineManifest) -> Result<(), CoreError> {
        let dir = self.mission_quarantine_dir(manifest.mission_id);
        fs::create_dir_all(&dir).map_err(|e| {
            CoreError::new(
                ErrorKind::App,
                "app.quarantine_failed",
                false,
                format!("Failed to create quarantine directory: {e}"),
                None,
            )
        })?;

        let json = serde_json::to_string_pretty(manifest).map_err(|e| {
            CoreError::new(
                ErrorKind::App,
                "app.quarantine_failed",
                false,
                format!("Failed to serialize quarantine manifest: {e}"),
                None,
            )
        })?;

        fs::write(self.manifest_path(manifest.mission_id), json).map_err(|e| {
            CoreError::new(
                ErrorKind::App,
                "app.quarantine_failed",
                false,
                format!("Failed to write quarantine manifest: {e}"),
                None,
            )
        })
    }

    /// Moves a file to the mission quarantine directory instead of deleting it permanently (Tier 3 safety).
    pub fn quarantine_file(
        &self,
        mission_id: i64,
        source_path: &Path,
        now_secs: u64,
    ) -> Result<QuarantinedItem, CoreError> {
        if !source_path.exists() {
            return Err(CoreError::new(
                ErrorKind::App,
                "app.file_not_found",
                false,
                format!(
                    "Cannot quarantine non-existent file: {}",
                    source_path.display()
                ),
                None,
            ));
        }

        let content = fs::read(source_path).map_err(|e| {
            CoreError::new(
                ErrorKind::App,
                "app.quarantine_failed",
                false,
                format!("Failed to read file for quarantine: {e}"),
                None,
            )
        })?;

        let mut hasher = Sha256::new();
        hasher.update(&content);
        let sha256 = hex::encode(hasher.finalize());

        let q_dir = self.mission_quarantine_dir(mission_id).join("files");
        fs::create_dir_all(&q_dir).map_err(|e| {
            CoreError::new(
                ErrorKind::App,
                "app.quarantine_failed",
                false,
                format!("Failed to create quarantine storage directory: {e}"),
                None,
            )
        })?;

        let file_name = source_path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("unnamed_file");
        let dest_filename = format!("{sha256}_{file_name}");
        let dest_path = q_dir.join(&dest_filename);

        // Move or copy-and-remove
        fs::rename(source_path, &dest_path)
            .or_else(|_| {
                fs::write(&dest_path, &content)?;
                fs::remove_file(source_path)?;
                Ok::<(), std::io::Error>(())
            })
            .map_err(|e| {
                CoreError::new(
                    ErrorKind::App,
                    "app.quarantine_failed",
                    false,
                    format!("Failed to move file to quarantine: {e}"),
                    None,
                )
            })?;

        let canonical_source = source_path.display().to_string();
        let item = QuarantinedItem {
            original_path: canonical_source.clone(),
            quarantine_path: dest_path.display().to_string(),
            sha256,
            size_bytes: content.len() as u64,
            quarantined_at_secs: now_secs,
        };

        let mut manifest = self.load_manifest(mission_id);
        if manifest.created_at_secs == 0 {
            manifest.created_at_secs = now_secs;
        }
        manifest.items.insert(canonical_source, item.clone());
        self.save_manifest(&manifest)?;

        Ok(item)
    }

    /// Restores a quarantined file back to its original path, verifying byte-identity via SHA-256.
    pub fn restore_file(&self, mission_id: i64, original_path: &str) -> Result<bool, CoreError> {
        let mut manifest = self.load_manifest(mission_id);
        let Some(item) = manifest.items.remove(original_path) else {
            return Err(CoreError::new(
                ErrorKind::App,
                "app.quarantine_not_found",
                false,
                format!("No quarantined item found for path '{original_path}'"),
                None,
            ));
        };

        let q_file = Path::new(&item.quarantine_path);
        if !q_file.exists() {
            return Err(CoreError::new(
                ErrorKind::App,
                "app.quarantine_missing",
                false,
                format!("Quarantine storage file missing: {}", item.quarantine_path),
                None,
            ));
        }

        let content = fs::read(q_file).map_err(|e| {
            CoreError::new(
                ErrorKind::App,
                "app.quarantine_failed",
                false,
                format!("Failed to read quarantined file: {e}"),
                None,
            )
        })?;

        // Verify SHA256 integrity
        let mut hasher = Sha256::new();
        hasher.update(&content);
        let actual_hash = hex::encode(hasher.finalize());
        if actual_hash != item.sha256 {
            return Err(CoreError::new(
                ErrorKind::App,
                "app.quarantine_corrupted",
                false,
                format!(
                    "Quarantined file hash mismatch for '{}'",
                    item.original_path
                ),
                None,
            ));
        }

        // Restore file
        let target = Path::new(&item.original_path);
        if let Some(parent) = target.parent() {
            let _ = fs::create_dir_all(parent);
        }

        fs::rename(q_file, target)
            .or_else(|_| {
                fs::write(target, &content)?;
                fs::remove_file(q_file)?;
                Ok::<(), std::io::Error>(())
            })
            .map_err(|e| {
                CoreError::new(
                    ErrorKind::App,
                    "app.quarantine_failed",
                    false,
                    format!("Failed to restore file from quarantine: {e}"),
                    None,
                )
            })?;

        self.save_manifest(&manifest)?;
        Ok(true)
    }

    /// Identifies whether an operation is fundamentally irreversible (e.g. package cache purges).
    #[must_use]
    pub fn is_irreversible(tool_name: &str) -> bool {
        let lower = tool_name.to_ascii_lowercase();
        lower.contains("purge_package_cache")
            || lower.contains("clean_cache")
            || lower.contains("format_disk")
            || lower.contains("wipe_storage")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_quarantine_and_restore_round_trip_byte_identical() {
        let tmp = tempdir().expect("tempdir");
        let quarantine_dir = tmp.path().join("quarantine");
        let manager = QuarantineManager::new(quarantine_dir);

        let work_dir = tmp.path().join("work");
        fs::create_dir_all(&work_dir).expect("create work dir");
        let test_file = work_dir.join("critical_take.wav");
        let original_bytes = b"RIFF....WAVEfmt ....data[Original Studio Voiceover 96kHz 24bit]";
        fs::write(&test_file, original_bytes).expect("write original");

        let mission_id = 42;
        let now_secs = 1_700_000_000;

        // 1. Quarantine file instead of silent delete
        let quarantined = manager
            .quarantine_file(mission_id, &test_file, now_secs)
            .expect("quarantine file");
        assert!(
            !test_file.exists(),
            "Original file must be removed from work directory"
        );
        assert!(
            Path::new(&quarantined.quarantine_path).exists(),
            "File must exist in quarantine storage"
        );
        assert_eq!(quarantined.size_bytes, original_bytes.len() as u64);

        // 2. Restore file
        let restored = manager
            .restore_file(mission_id, &test_file.display().to_string())
            .expect("restore file");
        assert!(restored);
        assert!(
            test_file.exists(),
            "Restored file must exist at original path"
        );

        // 3. Verify restored file is byte-identical
        let restored_bytes = fs::read(&test_file).expect("read restored");
        assert_eq!(restored_bytes, original_bytes);

        // 4. Verify irreversible cleaner detection
        assert!(QuarantineManager::is_irreversible(
            "atp.purge_package_cache"
        ));
        assert!(QuarantineManager::is_irreversible("system.clean_cache"));
        assert!(!QuarantineManager::is_irreversible("timeline.cut"));
        assert!(!QuarantineManager::is_irreversible("files.quarantine"));
    }
}
