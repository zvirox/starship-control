use crate::CoreError;
use sha2::{Digest, Sha256};

/// Secret scrubber and cipher vault ensuring secrets are never leaked into logs,
/// LLM contexts, or stored in plaintext in SQLite (GAP-CLOSURE-PLAN.md §10.7).
pub struct SecretVault;

impl SecretVault {
    /// Derives an encryption key from machine ID / salt.
    fn derive_vault_key(salt: &str) -> [u8; 32] {
        let mut hasher = Sha256::new();
        hasher.update(b"starship-vault-key-derivation-v1:");
        hasher.update(salt.as_bytes());
        hasher.finalize().into()
    }

    /// Encrypts a plaintext secret into an armored ciphertext format `enc_v1:<hex>`.
    /// Ensures that SQLite never stores raw API keys in plaintext (GAP-CLOSURE-PLAN.md §10.7).
    #[must_use]
    pub fn encrypt_secret(plaintext: &str, salt: &str) -> String {
        let key = Self::derive_vault_key(salt);
        let plain_bytes = plaintext.as_bytes();
        let mut cipher_bytes = Vec::with_capacity(plain_bytes.len());

        for (i, byte) in plain_bytes.iter().enumerate() {
            cipher_bytes.push(byte ^ key[i % key.len()]);
        }

        let hex_str = hex::encode(cipher_bytes);
        format!("enc_v1:{hex_str}")
    }

    /// Decrypts an armored ciphertext format `enc_v1:<hex>`.
    pub fn decrypt_secret(armored: &str, salt: &str) -> Result<String, CoreError> {
        let hex_part = armored.strip_prefix("enc_v1:").ok_or_else(|| {
            CoreError::safety(
                "safety.secret_decrypt_error",
                false,
                "Invalid armored secret format (missing enc_v1 prefix)",
                None,
            )
        })?;

        let cipher_bytes = hex::decode(hex_part).map_err(|e| {
            CoreError::safety(
                "safety.secret_decrypt_error",
                false,
                format!("Failed to decode ciphertext hex: {e}"),
                None,
            )
        })?;

        let key = Self::derive_vault_key(salt);
        let mut plain_bytes = Vec::with_capacity(cipher_bytes.len());

        for (i, byte) in cipher_bytes.iter().enumerate() {
            plain_bytes.push(byte ^ key[i % key.len()]);
        }

        String::from_utf8(plain_bytes).map_err(|e| {
            CoreError::safety(
                "safety.secret_decrypt_error",
                false,
                format!("Decrypted secret is not valid UTF-8: {e}"),
                None,
            )
        })
    }
}

/// Redacts sensitive API keys and secrets from logs and prompts.
pub struct SecretScrubber;

impl SecretScrubber {
    /// Scrubs known API keys and registered secrets from any text (prompts, logs, tool dumps).
    #[must_use]
    pub fn scrub(input: &str, known_secrets: &[&str]) -> String {
        let mut result = input.to_string();

        // 1. Scrub exact known secrets
        for &secret in known_secrets {
            let trimmed = secret.trim();
            if trimmed.len() >= 8 {
                result = result.replace(trimmed, "[REDACTED_API_KEY]");
            }
        }

        // 2. Pattern scrub common API key formats (sk-..., gsk_..., etc.)
        // Look for tokens starting with sk- and replace alphanumeric runs >= 20 chars
        result = Self::mask_pattern(&result, "sk-");
        result = Self::mask_pattern(&result, "gsk_");

        result
    }

    fn mask_pattern(text: &str, prefix: &str) -> String {
        let mut output = String::new();
        let mut rest = text;

        while let Some(idx) = rest.find(prefix) {
            output.push_str(&rest[..idx]);
            let candidate_start = &rest[idx..];

            // Find end of token
            let token_len = candidate_start
                .char_indices()
                .take_while(|(_, c)| c.is_ascii_alphanumeric() || *c == '-' || *c == '_')
                .last()
                .map_or(0, |(i, c)| i + c.len_utf8());

            if token_len >= 16 {
                output.push_str("[REDACTED_API_KEY]");
            } else {
                output.push_str(&candidate_start[..token_len]);
            }

            rest = &candidate_start[token_len..];
        }

        output.push_str(rest);
        output
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_canary_key_never_appears_in_plaintext_or_prompts() {
        let canary_key = "sk-ant-api03-canary999988887777666655554444333322221111";
        let salt = "test-system-salt-uuid-001";

        // 1. Test encryption at rest: Ciphertext must NOT contain canary plaintext
        let encrypted = SecretVault::encrypt_secret(canary_key, salt);
        assert!(encrypted.starts_with("enc_v1:"));
        assert!(
            !encrypted.contains(canary_key),
            "Security failure: encrypted ciphertext contains canary key plaintext!"
        );

        // Decryption round-trip recovers original
        let decrypted = SecretVault::decrypt_secret(&encrypted, salt).unwrap();
        assert_eq!(decrypted, canary_key);

        // 2. Test scrubber on log / prompt text
        let prompt_with_key = format!(
            "System instruction: Call API endpoint using header Authorization: Bearer {canary_key} to fetch data."
        );

        let scrubbed = SecretScrubber::scrub(&prompt_with_key, &[canary_key]);

        assert!(
            !scrubbed.contains(canary_key),
            "Security failure: scrubbed prompt still contains canary key!"
        );
        assert!(
            scrubbed.contains("[REDACTED_API_KEY]"),
            "Scrubbed prompt missing redaction marker"
        );
    }
}
