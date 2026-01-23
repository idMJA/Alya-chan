use chrono::{DateTime, Utc};
use libsql::{params, Builder, Database};
use std::path::Path;
use tokio::sync::OnceCell;
use uuid::Uuid;

/// Hybrid local (preferred) + remote (Turso/libsql) store.
/// Writes always go to local; remote sync is best-effort.
pub struct HybridStore {
    local: Database,
    remote: Option<Database>,
}

static STORE: OnceCell<HybridStore> = OnceCell::const_new();

impl HybridStore {
    /// Initialize the store (singleton).
    pub async fn init(
        local_path: &str,
        remote_url: Option<&str>,
        remote_token: Option<&str>,
    ) -> anyhow::Result<&'static HybridStore> {
        STORE
            .get_or_try_init(|| async {
                // Ensure local directory exists
                if let Some(parent) = Path::new(local_path).parent() {
                    std::fs::create_dir_all(parent)?;
                }

                // Local DB
                let local = Builder::new_local(local_path).build().await?;

                // Optional remote DB (Turso/libsql)
                let remote = if let Some(url) = remote_url {
                    let token = remote_token.unwrap_or("");
                    Some(
                        Builder::new_remote(url.to_string(), token.to_string())
                            .build()
                            .await?,
                    )
                } else {
                    None
                };

                let store = Self { local, remote };
                store.ensure_schema().await?;
                Ok(store)
            })
            .await
    }

    /// Create table if not exists on both local and remote.
    async fn ensure_schema(&self) -> anyhow::Result<()> {
        const SCHEMA: &str = r#"
            CREATE TABLE IF NOT EXISTS chat_history (
                id TEXT PRIMARY KEY,
                user_id TEXT NOT NULL,
                guild_id TEXT NOT NULL,
                role TEXT NOT NULL,
                content TEXT NOT NULL,
                created_at TEXT NOT NULL
            );
        "#;

        let conn = self.local.connect()?;
        conn.execute(SCHEMA, ()).await?;

        if let Some(remote) = &self.remote {
            if let Ok(conn) = remote.connect() {
                let _ = conn.execute(SCHEMA, ()).await;
            }
        }

        Ok(())
    }

    /// Append a message to local, then best-effort to remote.
    pub async fn append_message(
        &self,
        user_id: &str,
        guild_id: &str,
        role: &str,
        content: &str,
        created_at: DateTime<Utc>,
    ) -> anyhow::Result<()> {
        let id = Uuid::new_v4().to_string();
        let created_iso = created_at.to_rfc3339();

        let conn = self.local.connect()?;
        conn.execute(
            "INSERT INTO chat_history (id, user_id, guild_id, role, content, created_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![id, user_id, guild_id, role, content, created_iso],
        )
        .await?;

        if let Some(remote) = &self.remote {
            if let Ok(conn) = remote.connect() {
                let created_iso_remote = created_at.to_rfc3339();
                let _ = conn
                    .execute(
                        "INSERT INTO chat_history (id, user_id, guild_id, role, content, created_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
                        params![Uuid::new_v4().to_string(), user_id, guild_id, role, content, created_iso_remote],
                    )
                    .await;
            }
        }

        Ok(())
    }

    /// Fetch last `limit` messages for a user within `ttl_minutes` window, ordered oldest->newest.
    pub async fn fetch_recent(
        &self,
        user_id: &str,
        ttl_minutes: i64,
        limit: usize,
    ) -> anyhow::Result<Vec<(String, String)>> {
        let threshold = Utc::now() - chrono::Duration::minutes(ttl_minutes);
        let threshold_iso = threshold.to_rfc3339();

        let conn = self.local.connect()?;
        let mut rows = conn
            .query(
                "SELECT role, content FROM chat_history WHERE user_id = ?1 AND created_at >= ?2 ORDER BY datetime(created_at) ASC LIMIT ?3",
                params![user_id, threshold_iso, limit as i64],
            )
            .await?;

        let mut result = Vec::new();
        while let Some(row) = rows.next().await? {
            let role: String = row.get(0)?;
            let content: String = row.get(1)?;
            result.push((role, content));
        }

        Ok(result)
    }
}
