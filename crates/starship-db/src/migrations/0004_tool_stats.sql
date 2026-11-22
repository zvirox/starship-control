-- Migration 0004: Tool Stats Learner (GAP-CLOSURE-PLAN.md §13.3 & Appendix B)

CREATE TABLE IF NOT EXISTS tool_stats (
  app_id TEXT NOT NULL,
  tool TEXT NOT NULL,
  n INTEGER NOT NULL DEFAULT 0,
  ewma_ms REAL NOT NULL DEFAULT 0.0,
  p50_ms REAL NOT NULL DEFAULT 0.0,
  p90_ms REAL NOT NULL DEFAULT 0.0,
  samples_json TEXT NOT NULL DEFAULT '[]',
  PRIMARY KEY (app_id, tool)
);

CREATE INDEX IF NOT EXISTS idx_tool_stats_tool ON tool_stats(tool);
