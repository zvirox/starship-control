use crate::Database;
use rusqlite::params;
use serde::{Deserialize, Serialize};
use starship_core::CoreError;

/// Discovered application record stored in SQLite.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct AppRecord {
    pub app_id: String,
    pub display_name: String,
    pub version: String,
    pub binary_path: String,
    pub manifest_path: String,
    pub socket_path: Option<String>,
    pub capabilities_json: String,
    pub is_active_gui: bool,
    pub last_seen_at: String,
    pub created_at: String,
}

impl Database {
    /// Inserts or updates an application entry in the `app_registry` table.
    pub fn upsert_app(&self, app: &AppRecord) -> Result<(), CoreError> {
        self.with_conn(|conn| {
            conn.execute(
                "INSERT INTO app_registry (
                    app_id, display_name, version, binary_path, manifest_path,
                    socket_path, capabilities_json, is_active_gui, last_seen_at
                ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)
                ON CONFLICT(app_id) DO UPDATE SET
                    display_name = excluded.display_name,
                    version = excluded.version,
                    binary_path = excluded.binary_path,
                    manifest_path = excluded.manifest_path,
                    socket_path = excluded.socket_path,
                    capabilities_json = excluded.capabilities_json,
                    is_active_gui = excluded.is_active_gui,
                    last_seen_at = excluded.last_seen_at",
                params![
                    app.app_id,
                    app.display_name,
                    app.version,
                    app.binary_path,
                    app.manifest_path,
                    app.socket_path,
                    app.capabilities_json,
                    i64::from(app.is_active_gui),
                    app.last_seen_at,
                ],
            )?;
            Ok(())
        })
    }

    /// Fetches an application record by its unique `app_id`.
    pub fn get_app(&self, app_id: &str) -> Result<Option<AppRecord>, CoreError> {
        self.with_conn(|conn| {
            let mut stmt = conn.prepare(
                "SELECT app_id, display_name, version, binary_path, manifest_path,
                        socket_path, capabilities_json, is_active_gui, last_seen_at, created_at
                 FROM app_registry WHERE app_id = ?1",
            )?;

            let mut rows = stmt.query(params![app_id])?;
            if let Some(row) = rows.next()? {
                let is_active_gui: i64 = row.get(7)?;
                Ok(Some(AppRecord {
                    app_id: row.get(0)?,
                    display_name: row.get(1)?,
                    version: row.get(2)?,
                    binary_path: row.get(3)?,
                    manifest_path: row.get(4)?,
                    socket_path: row.get(5)?,
                    capabilities_json: row.get(6)?,
                    is_active_gui: is_active_gui != 0,
                    last_seen_at: row.get(8)?,
                    created_at: row.get(9)?,
                }))
            } else {
                Ok(None)
            }
        })
    }

    /// Lists all applications registered in the `app_registry` table.
    pub fn list_apps(&self) -> Result<Vec<AppRecord>, CoreError> {
        self.with_conn(|conn| {
            let mut stmt = conn.prepare(
                "SELECT app_id, display_name, version, binary_path, manifest_path,
                        socket_path, capabilities_json, is_active_gui, last_seen_at, created_at
                 FROM app_registry ORDER BY app_id ASC",
            )?;

            let rows = stmt.query_map([], |row| {
                let is_active_gui: i64 = row.get(7)?;
                Ok(AppRecord {
                    app_id: row.get(0)?,
                    display_name: row.get(1)?,
                    version: row.get(2)?,
                    binary_path: row.get(3)?,
                    manifest_path: row.get(4)?,
                    socket_path: row.get(5)?,
                    capabilities_json: row.get(6)?,
                    is_active_gui: is_active_gui != 0,
                    last_seen_at: row.get(8)?,
                    created_at: row.get(9)?,
                })
            })?;

            let mut apps = Vec::new();
            for r in rows {
                apps.push(r?);
            }
            Ok(apps)
        })
    }

    /// Updates an application's live GUI connection status and socket path.
    pub fn update_app_gui_status(
        &self,
        app_id: &str,
        is_active_gui: bool,
        socket_path: Option<&str>,
    ) -> Result<(), CoreError> {
        self.with_conn(|conn| {
            conn.execute(
                "UPDATE app_registry SET
                    is_active_gui = ?1,
                    socket_path = ?2,
                    last_seen_at = strftime('%Y-%m-%dT%H:%M:%SZ', 'now')
                 WHERE app_id = ?3",
                params![i64::from(is_active_gui), socket_path, app_id],
            )?;
            Ok(())
        })
    }

    /// Removes an application record from the registry table.
    pub fn remove_app(&self, app_id: &str) -> Result<(), CoreError> {
        self.with_conn(|conn| {
            conn.execute(
                "DELETE FROM app_registry WHERE app_id = ?1",
                params![app_id],
            )?;
            Ok(())
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_app_registry_crud() {
        let db = Database::open_in_memory().expect("open in memory");

        let app = AppRecord {
            app_id: "eep".to_string(),
            display_name: "Easy Edits Pro".to_string(),
            version: "0.1.2-alpha.7".to_string(),
            binary_path: "/usr/bin/easy-edits-pro".to_string(),
            manifest_path: "/home/user/.local/share/starship/plugins/eep.json".to_string(),
            socket_path: Some("/run/user/1000/starship/bridges/eep.sock".to_string()),
            capabilities_json: r#"{"videoEditing":true}"#.to_string(),
            is_active_gui: false,
            last_seen_at: "2026-11-22T10:00:00Z".to_string(),
            created_at: "2026-11-22T10:00:00Z".to_string(),
        };

        db.upsert_app(&app).expect("upsert");

        let fetched = db.get_app("eep").expect("get_app").expect("found");
        assert_eq!(fetched.display_name, "Easy Edits Pro");
        assert!(!fetched.is_active_gui);

        db.update_app_gui_status(
            "eep",
            true,
            Some("/run/user/1000/starship/bridges/eep.sock"),
        )
        .expect("update status");

        let updated = db.get_app("eep").expect("get_app").expect("found");
        assert!(updated.is_active_gui);

        let list = db.list_apps().expect("list_apps");
        assert_eq!(list.len(), 1);

        db.remove_app("eep").expect("remove_app");
        let empty = db.get_app("eep").expect("get_app");
        assert!(empty.is_none());
    }
}
