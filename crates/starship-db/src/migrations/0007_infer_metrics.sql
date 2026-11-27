-- Per-request inference telemetry (Phase 17, task 17.5).
-- All values are measured in the stream loop; no preset or simulated data.
CREATE TABLE IF NOT EXISTS infer_metrics (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  ts_ms INTEGER NOT NULL,
  mission_id INTEGER,
  provider TEXT NOT NULL,
  model_id TEXT NOT NULL,
  first_token_ms INTEGER NOT NULL DEFAULT 0,
  total_latency_ms INTEGER NOT NULL DEFAULT 0,
  tokens_in INTEGER NOT NULL DEFAULT 0,
  tokens_out INTEGER NOT NULL DEFAULT 0,
  tokens_per_sec REAL NOT NULL DEFAULT 0,
  error_code TEXT
);

CREATE INDEX IF NOT EXISTS idx_infer_metrics_ts ON infer_metrics(ts_ms);
