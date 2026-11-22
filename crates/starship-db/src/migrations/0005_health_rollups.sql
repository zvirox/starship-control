-- Migration 0005: Health Rollups and Alerts Storage (GAP-CLOSURE-PLAN.md §14.5 & Appendix B)

CREATE TABLE IF NOT EXISTS health_rollup_1m (
  ts INTEGER PRIMARY KEY,
  cpu_avg REAL NOT NULL,
  cpu_max REAL NOT NULL,
  mem_avail_pct_min REAL NOT NULL,
  swap_in_max REAL NOT NULL,
  disk_free_pct_min REAL NOT NULL,
  temp_max REAL NOT NULL,
  gpu_util_max REAL,
  vram_pct_max REAL,
  psi_cpu_max REAL,
  psi_mem_max REAL,
  psi_io_max REAL,
  state TEXT NOT NULL
);
CREATE INDEX IF NOT EXISTS idx_health_rollup_1m_ts ON health_rollup_1m(ts);

CREATE TABLE IF NOT EXISTS health_rollup_1h (
  ts INTEGER PRIMARY KEY,
  cpu_avg REAL NOT NULL,
  cpu_max REAL NOT NULL,
  mem_avail_pct_min REAL NOT NULL,
  swap_in_max REAL NOT NULL,
  disk_free_pct_min REAL NOT NULL,
  temp_max REAL NOT NULL,
  gpu_util_max REAL,
  vram_pct_max REAL,
  psi_cpu_max REAL,
  psi_mem_max REAL,
  psi_io_max REAL,
  state TEXT NOT NULL
);
CREATE INDEX IF NOT EXISTS idx_health_rollup_1h_ts ON health_rollup_1h(ts);

CREATE TABLE IF NOT EXISTS health_alerts (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  ts INTEGER NOT NULL,
  signal TEXT NOT NULL,
  level TEXT NOT NULL,
  value REAL NOT NULL,
  action TEXT NOT NULL,
  mission_id INTEGER REFERENCES missions(id) ON DELETE SET NULL
);
CREATE INDEX IF NOT EXISTS idx_health_alerts_ts ON health_alerts(ts);
