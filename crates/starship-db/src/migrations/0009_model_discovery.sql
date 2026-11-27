-- Provider-discovered model metadata (Phase 17, 17.21).
ALTER TABLE models ADD COLUMN provider_id TEXT;
ALTER TABLE models ADD COLUMN supports_tools INTEGER NOT NULL DEFAULT 0;
ALTER TABLE models ADD COLUMN supports_streaming INTEGER NOT NULL DEFAULT 1;
ALTER TABLE models ADD COLUMN input_price REAL NOT NULL DEFAULT 0;
ALTER TABLE models ADD COLUMN output_price REAL NOT NULL DEFAULT 0;
ALTER TABLE models ADD COLUMN is_favorite INTEGER NOT NULL DEFAULT 0;
ALTER TABLE models ADD COLUMN discovered_at TEXT;

CREATE INDEX IF NOT EXISTS idx_models_provider ON models(provider_id);
