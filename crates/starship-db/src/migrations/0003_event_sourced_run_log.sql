-- Migration 0003: Event-Sourced Run Log & Step State (GAP-CLOSURE-PLAN.md §11.2 & Appendix B)

CREATE TABLE IF NOT EXISTS mission_events (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  mission_id INTEGER NOT NULL REFERENCES missions(id) ON DELETE CASCADE,
  step_id INTEGER,
  ts_ms INTEGER NOT NULL,
  kind TEXT NOT NULL,
  payload_json TEXT NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_events_mission ON mission_events(mission_id, id);

ALTER TABLE plan_steps ADD COLUMN state TEXT NOT NULL DEFAULT 'pending';
ALTER TABLE plan_steps ADD COLUMN attempt INTEGER NOT NULL DEFAULT 0;
ALTER TABLE plan_steps ADD COLUMN checkpoint_id TEXT;
ALTER TABLE plan_steps ADD COLUMN optional INTEGER NOT NULL DEFAULT 0;
ALTER TABLE plan_steps ADD COLUMN idempotency_key TEXT;
ALTER TABLE plan_steps ADD COLUMN progress REAL NOT NULL DEFAULT 0.0;
ALTER TABLE plan_steps ADD COLUMN failure_class TEXT;
