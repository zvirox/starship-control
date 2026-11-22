use sha2::{Digest, Sha256};
use starship_core::CoreError;
use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};

/// Security verification for plugin manifests and socket peers (GAP-CLOSURE-PLAN.md §10.5, §10.6).
pub struct SecurityVerifier;

impl SecurityVerifier {
    /// Validates binary path according to security constraints (GAP-CLOSURE-PLAN.md §10.5):
    /// - Must be absolute
    /// - Must NOT be located in `/tmp` or `/var/tmp`
    /// - Must NOT be located in `$HOME/Downloads`
    /// - Must NOT be world-writable
    pub fn validate_binary_path(path: &Path) -> Result<(), CoreError> {
        if !path.is_absolute() {
            return Err(CoreError::mcp(
                "mcp.manifest_untrusted",
                false,
                format!("Binary path must be absolute: {}", path.display()),
                None,
            ));
        }

        if path.starts_with("/tmp") || path.starts_with("/var/tmp") {
            return Err(CoreError::mcp(
                "mcp.manifest_untrusted",
                false,
                format!(
                    "Binary cannot be located in temporary directory: {}",
                    path.display()
                ),
                None,
            ));
        }

        if let Ok(home) = std::env::var("HOME") {
            let downloads = PathBuf::from(home).join("Downloads");
            if path.starts_with(&downloads) {
                return Err(CoreError::mcp(
                    "mcp.manifest_untrusted",
                    false,
                    format!(
                        "Binary cannot be located in Downloads directory: {}",
                        path.display()
                    ),
                    None,
                ));
            }
        }

        if let Ok(meta) = fs::metadata(path) {
            let mode = meta.permissions().mode();
            if mode & 0o002 != 0 {
                return Err(CoreError::mcp(
                    "mcp.manifest_untrusted",
                    false,
                    format!("Binary must not be world-writable: {}", path.display()),
                    None,
                ));
            }
        }

        Ok(())
    }

    /// Computes the SHA-256 hash of a file at the given path.
    pub fn compute_binary_hash(path: &Path) -> Result<String, CoreError> {
        let bytes = fs::read(path).map_err(|e| {
            CoreError::mcp(
                "mcp.fs_error",
                false,
                format!("Failed to read binary for hash computation: {e}"),
                None,
            )
        })?;
        let mut hasher = Sha256::new();
        hasher.update(&bytes);
        Ok(hex::encode(hasher.finalize()))
    }

    /// Verifies binary pinning (GAP-CLOSURE-PLAN.md §10.5):
    /// If an expected pinned hash is supplied, the current binary hash must match.
    pub fn verify_pinned_hash(
        path: &Path,
        expected_pinned_hash: Option<&str>,
    ) -> Result<String, CoreError> {
        Self::validate_binary_path(path)?;
        let current_hash = Self::compute_binary_hash(path)?;

        if let Some(expected) = expected_pinned_hash
            && !current_hash.eq_ignore_ascii_case(expected)
        {
            return Err(CoreError::mcp(
                "mcp.manifest_untrusted",
                false,
                format!(
                    "Tampered binary detected for {}: expected SHA256 {expected}, got {current_hash}",
                    path.display()
                ),
                Some(serde_json::json!({
                    "path": path.display().to_string(),
                    "expectedHash": expected,
                    "actualHash": current_hash,
                })),
            ));
        }

        Ok(current_hash)
    }

    /// Inspects the Linux `/proc/<pid>/exe` symlink and compares it against the pinned binary path.
    /// Rejects connections from mismatched binaries (GAP-CLOSURE-PLAN.md §10.6).
    pub fn verify_peer_executable(
        peer_pid: i32,
        pinned_binary_path: &Path,
    ) -> Result<(), CoreError> {
        let exe_link = format!("/proc/{peer_pid}/exe");
        let resolved = fs::read_link(&exe_link).map_err(|e| {
            CoreError::mcp(
                "mcp.so_peercred_failed",
                false,
                format!("Failed to inspect peer executable via {exe_link}: {e}"),
                Some(serde_json::json!({
                    "peerPid": peer_pid,
                    "pinnedPath": pinned_binary_path.display().to_string(),
                })),
            )
        })?;

        let canonical_pinned = fs::canonicalize(pinned_binary_path)
            .unwrap_or_else(|_| pinned_binary_path.to_path_buf());
        let canonical_resolved = fs::canonicalize(&resolved).unwrap_or(resolved);

        if canonical_resolved != canonical_pinned {
            return Err(CoreError::mcp(
                "mcp.so_peercred_failed",
                false,
                format!(
                    "Security violation: peer process {peer_pid} executable '{}' does not match pinned manifest binaryPath '{}'",
                    canonical_resolved.display(),
                    canonical_pinned.display()
                ),
                Some(serde_json::json!({
                    "peerPid": peer_pid,
                    "actualExe": canonical_resolved.display().to_string(),
                    "pinnedExe": canonical_pinned.display().to_string(),
                })),
            ));
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_manifest_path_validation() {
        // Relative path rejected
        assert_eq!(
            SecurityVerifier::validate_binary_path(Path::new("relative/bin"))
                .unwrap_err()
                .code,
            "mcp.manifest_untrusted"
        );

        // /tmp path rejected
        assert_eq!(
            SecurityVerifier::validate_binary_path(Path::new("/tmp/evil_tool"))
                .unwrap_err()
                .code,
            "mcp.manifest_untrusted"
        );

        // /var/tmp path rejected
        assert_eq!(
            SecurityVerifier::validate_binary_path(Path::new("/var/tmp/evil_tool"))
                .unwrap_err()
                .code,
            "mcp.manifest_untrusted"
        );

        // Valid system path accepted
        assert!(SecurityVerifier::validate_binary_path(Path::new("/usr/bin/starship-app")).is_ok());
    }

    #[test]
    fn test_tampered_manifest_refuses_spawn() {
        // Create test file in target/ (outside /tmp to pass validate_binary_path)
        let _ = fs::create_dir_all("target/test_bin");
        let test_bin_path = PathBuf::from("target/test_bin/test_exec.bin");
        let canonical_bin = fs::canonicalize(PathBuf::from("."))
            .unwrap()
            .join(&test_bin_path);
        fs::write(&canonical_bin, b"original binary code").unwrap();

        let original_hash = SecurityVerifier::compute_binary_hash(&canonical_bin).unwrap();

        // Matching hash succeeds
        assert!(SecurityVerifier::verify_pinned_hash(&canonical_bin, Some(&original_hash)).is_ok());

        // Tampered hash refuses spawn with mcp.manifest_untrusted
        let tampered_result = SecurityVerifier::verify_pinned_hash(
            &canonical_bin,
            Some("bad_hash_0000000000000000000000000000000000000000000000000000000000000000"),
        );
        assert!(tampered_result.is_err());
        let err = tampered_result.unwrap_err();
        assert_eq!(err.code, "mcp.manifest_untrusted");
        assert!(err.user_message.contains("Tampered binary detected"));

        let _ = fs::remove_file(&canonical_bin);
    }

    #[test]
    fn test_peer_binary_mismatch_rejected() {
        let current_pid = i32::try_from(std::process::id()).unwrap_or(0);
        let dummy_wrong_path = Path::new("/usr/bin/non_matching_binary_name");

        let result = SecurityVerifier::verify_peer_executable(current_pid, dummy_wrong_path);
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert_eq!(err.code, "mcp.so_peercred_failed");
        assert!(err.user_message.contains("does not match pinned manifest"));
    }
}
