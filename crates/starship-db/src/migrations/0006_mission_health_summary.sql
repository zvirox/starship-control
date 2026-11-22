-- Migration 0006: Mission Health & Governor Intervention summary fields (GAP-CLOSURE-PLAN.md §14.11)
ALTER TABLE missions ADD COLUMN peak_cpu_pct REAL NOT NULL DEFAULT 0.0;
ALTER TABLE missions ADD COLUMN peak_ram_bytes INTEGER NOT NULL DEFAULT 0;
ALTER TABLE missions ADD COLUMN peak_temp_celsius REAL NOT NULL DEFAULT 0.0;
ALTER TABLE missions ADD COLUMN governor_interventions INTEGER NOT NULL DEFAULT 0;
