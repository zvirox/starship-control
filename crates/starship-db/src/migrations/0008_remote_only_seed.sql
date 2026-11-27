-- The seeded qwen model has no GGUF file. Mark it remote_only so it can
-- never appear as an installed, ready local model (Phase 17, 17.15).
UPDATE models SET download_status = 'remote_only', file_path = NULL
 WHERE id = 'qwen2.5-coder-7b-q4' AND file_path IS NULL;
