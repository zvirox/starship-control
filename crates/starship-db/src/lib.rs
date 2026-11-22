use rusqlite::Connection;
use starship_core::CoreError;
use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use tracing::{info, warn};

pub mod apps;
pub mod audit;
pub mod events;
pub mod models;
pub mod plans;
pub mod schema;
pub mod tool_stats;

pub use apps::AppRecord;
pub use audit::{AuditRecord, NewAuditEntry};
pub use events::{MissionEventRecord, MissionHistorySummary, NewMissionEvent};
pub use plans::{PlanRecord, PlanStepRecord};
pub use tool_stats::ToolStatsRecord;

const INITIAL_MIGRATION: &str = include_str!("migrations/0001_initial.sql");
const MIGRATION_0002: &str = include_str!("migrations/0002_tamper_evident_audit_log.sql");
const MIGRATION_0003: &str = include_str!("migrations/0003_event_sourced_run_log.sql");
const MIGRATION_0004: &str = include_str!("migrations/0004_tool_stats.sql");
const CURRENT_SCHEMA_VERSION: u32 = 4;

#[derive(Debug, Clone)]
pub struct Database {
    conn: Arc<Mutex<Connection>>,
    db_path: Option<PathBuf>,
}

impl Database {
    /// Opens or creates the SQLite database at the standard XDG path:
    /// `$XDG_DATA_HOME/starship-control/library.db`.
    /// Sets strict permissions (0700 dir, 0600 file) and runs migrations.
    pub fn open_default() -> Result<Self, CoreError> {
        let base_dir = dirs::data_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("starship-control");

        if !base_dir.exists() {
            fs::create_dir_all(&base_dir).map_err(|e| {
                CoreError::db(
                    "db.init_failed",
                    false,
                    format!("Failed to create data directory: {e}"),
                    None,
                )
            })?;
            // Set 0700 directory permissions
            let _ = fs::set_permissions(&base_dir, fs::Permissions::from_mode(0o700));
        }

        let db_path = base_dir.join("library.db");
        Self::open_path(db_path)
    }

    /// Opens or creates a SQLite database at a custom path (useful for testing or custom configs).
    pub fn open_path<P: AsRef<Path>>(path: P) -> Result<Self, CoreError> {
        let path_buf = path.as_ref().to_path_buf();
        if let Some(parent) = path_buf.parent()
            && !parent.exists()
        {
            fs::create_dir_all(parent).map_err(|e| {
                CoreError::db(
                    "db.init_failed",
                    false,
                    format!("Failed to create directory {}: {e}", parent.display()),
                    None,
                )
            })?;
            let _ = fs::set_permissions(parent, fs::Permissions::from_mode(0o700));
        }

        let conn = Connection::open(&path_buf).map_err(|e| {
            CoreError::db(
                "db.open_failed",
                false,
                format!("Failed to open database at {}: {e}", path_buf.display()),
                None,
            )
        })?;

        // Set 0600 permissions on the created SQLite database file
        let _ = fs::set_permissions(&path_buf, fs::Permissions::from_mode(0o600));

        let db = Self {
            conn: Arc::new(Mutex::new(conn)),
            db_path: Some(path_buf),
        };

        db.configure_pragmas()?;
        db.run_migrations()?;

        Ok(db)
    }

    /// Creates an in-memory database instance (ideal for automated test isolation).
    pub fn open_in_memory() -> Result<Self, CoreError> {
        let conn = Connection::open_in_memory().map_err(|e| {
            CoreError::db(
                "db.open_failed",
                false,
                format!("Failed to open in-memory database: {e}"),
                None,
            )
        })?;

        let db = Self {
            conn: Arc::new(Mutex::new(conn)),
            db_path: None,
        };

        db.configure_pragmas()?;
        db.run_migrations()?;

        Ok(db)
    }

    /// Returns the database file path, if file-backed.
    #[must_use]
    pub fn path(&self) -> Option<&Path> {
        self.db_path.as_deref()
    }

    /// Configures SQLite pragmas strictly according to DATABASE.md §1:
    /// WAL mode, foreign keys enabled, busy timeout 5000ms.
    fn configure_pragmas(&self) -> Result<(), CoreError> {
        let conn = self.conn.lock().map_err(|_| {
            CoreError::db("db.lock_poisoned", false, "Database lock poisoned", None)
        })?;

        // If in-memory, WAL is not supported (returns error or no-op), but for file-backed it sets WAL
        if self.db_path.is_some() {
            let _ = conn.execute_batch("PRAGMA journal_mode = WAL;");
        }

        conn.execute_batch(
            "PRAGMA foreign_keys = ON;
             PRAGMA busy_timeout = 5000;",
        )
        .map_err(|e| {
            CoreError::db(
                "db.pragma_failed",
                false,
                format!("Failed to configure SQLite pragmas: {e}"),
                None,
            )
        })?;

        Ok(())
    }

    /// Executes versioned migrations using `PRAGMA user_version`.
    pub fn run_migrations(&self) -> Result<(), CoreError> {
        let conn = self.conn.lock().map_err(|_| {
            CoreError::db("db.lock_poisoned", false, "Database lock poisoned", None)
        })?;

        let current_version: u32 = conn
            .query_row("PRAGMA user_version", [], |row| row.get(0))
            .map_err(|e| {
                CoreError::db(
                    "db.read_failed",
                    false,
                    format!("Failed to query PRAGMA user_version: {e}"),
                    None,
                )
            })?;

        if current_version < 1 {
            info!(target: "starship_db", "Applying migration 0001_initial.sql");
            conn.execute_batch(INITIAL_MIGRATION).map_err(|e| {
                CoreError::db(
                    "db.migration_failed",
                    false,
                    format!("Failed to execute migration 0001: {e}"),
                    None,
                )
            })?;

            conn.execute_batch("PRAGMA user_version = 1;")
                .map_err(|e| {
                    CoreError::db(
                        "db.migration_failed",
                        false,
                        format!("Failed to set user_version to 1: {e}"),
                        None,
                    )
                })?;
        }

        if current_version < 2 {
            info!(target: "starship_db", "Applying migration 0002_tamper_evident_audit_log.sql");
            conn.execute_batch(MIGRATION_0002).map_err(|e| {
                CoreError::db(
                    "db.migration_failed",
                    false,
                    format!("Failed to execute migration 0002: {e}"),
                    None,
                )
            })?;

            conn.execute_batch("PRAGMA user_version = 2;")
                .map_err(|e| {
                    CoreError::db(
                        "db.migration_failed",
                        false,
                        format!("Failed to set user_version to 2: {e}"),
                        None,
                    )
                })?;
        }

        if current_version < 3 {
            info!(target: "starship_db", "Applying migration 0003_event_sourced_run_log.sql");
            conn.execute_batch(MIGRATION_0003).map_err(|e| {
                CoreError::db(
                    "db.migration_failed",
                    false,
                    format!("Failed to execute migration 0003: {e}"),
                    None,
                )
            })?;

            conn.execute_batch("PRAGMA user_version = 3;")
                .map_err(|e| {
                    CoreError::db(
                        "db.migration_failed",
                        false,
                        format!("Failed to set user_version to 3: {e}"),
                        None,
                    )
                })?;
        }

        if current_version < 4 {
            info!(target: "starship_db", "Applying migration 0004_tool_stats.sql");
            conn.execute_batch(MIGRATION_0004).map_err(|e| {
                CoreError::db(
                    "db.migration_failed",
                    false,
                    format!("Failed to execute migration 0004: {e}"),
                    None,
                )
            })?;

            conn.execute_batch("PRAGMA user_version = 4;")
                .map_err(|e| {
                    CoreError::db(
                        "db.migration_failed",
                        false,
                        format!("Failed to set user_version to 4: {e}"),
                        None,
                    )
                })?;
        }

        if current_version > CURRENT_SCHEMA_VERSION {
            warn!(
                target: "starship_db",
                "Database schema version {current_version} is newer than binary version {CURRENT_SCHEMA_VERSION}"
            );
        }

        Ok(())
    }

    /// Access the raw connection for transaction or query execution.
    pub fn with_conn<F, T>(&self, f: F) -> Result<T, CoreError>
    where
        F: FnOnce(&mut Connection) -> Result<T, rusqlite::Error>,
    {
        let mut conn = self.conn.lock().map_err(|_| {
            CoreError::db("db.lock_poisoned", false, "Database lock poisoned", None)
        })?;
        f(&mut conn).map_err(|e| {
            CoreError::db(
                "db.query_failed",
                false,
                format!("Database query failed: {e}"),
                None,
            )
        })
    }
}
