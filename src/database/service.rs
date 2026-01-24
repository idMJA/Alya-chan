use chrono::{DateTime, Duration, Utc};
use libsql::{params, Builder};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{OnceCell, RwLock};
use uuid::Uuid;

const DEFAULT_LOCALE: &str = "en-US";
const DEFAULT_PREFIX: &str = "!";
const VOTE_PREMIUM_DURATION_HOURS: i64 = 12;

pub struct ISetup {
    pub id: String,
    pub guild_id: String,
    pub channel_id: String,
    pub created_at: DateTime<Utc>,
}

pub struct PremiumStatus {
    pub vote_type: String,
    pub time_remaining: u64,
}

pub struct VoteStats {
    pub total: i64,
    pub active: i64,
}

#[derive(Clone)]
pub struct PremiumStats {
    pub active: bool,
    pub expires_at: Option<DateTime<Utc>>,
    pub vote_type: Option<String>,
}

pub struct AlyaDatabase {
    db: libsql::Database,
    cache: Arc<RwLock<HashMap<String, CachedGuild>>>,
    db_path: String,
    remote_url: Option<String>,
    remote_token: Option<String>,
}

#[derive(Clone, Default)]
struct CachedGuild {
    locale: Option<String>,
    prefix: Option<String>,
}

static DB: OnceCell<AlyaDatabase> = OnceCell::const_new();

impl AlyaDatabase {
    pub async fn init(
        local_path: &str,
        remote_url: Option<&str>,
        remote_token: Option<&str>,
    ) -> anyhow::Result<&'static AlyaDatabase> {
        DB.get_or_try_init(|| async {
            let db = if let Some(url) = remote_url {
                let token = remote_token.unwrap_or("");
                // Turso embedded replica: local sync + remote write
                Builder::new_remote_replica(local_path, url.to_string(), token.to_string())
                    .sync_interval(std::time::Duration::from_secs(300)) // Sync every 5 min
                    .build()
                    .await?
            } else {
                // Local only fallback
                Builder::new_local(local_path).build().await?
            };

            let alya_db = Self {
                db,
                cache: Arc::new(RwLock::new(HashMap::new())),
                db_path: local_path.to_string(),
                remote_url: remote_url.map(ToString::to_string),
                remote_token: remote_token.map(ToString::to_string),
            };

            alya_db.ensure_schema().await?;
            Ok(alya_db)
        })
        .await
    }

    pub fn get() -> anyhow::Result<&'static AlyaDatabase> {
        DB.get()
            .ok_or_else(|| anyhow::anyhow!("AlyaDatabase is not initialized"))
    }

    /// Manually sync local replica from remote (call on bot startup)
    pub async fn sync(&self) -> anyhow::Result<()> {
        self.db.sync().await?;
        tracing::info!("Database synced from remote");
        Ok(())
    }

    async fn ensure_schema(&self) -> anyhow::Result<()> {
        let conn = self.db.connect()?;

        const GUILD_SCHEMA: &str = r#"
            CREATE TABLE IF NOT EXISTS guild (
                id TEXT PRIMARY KEY,
                locale TEXT,
                prefix TEXT,
                chatbot_channel_id TEXT,
                global_channel_id TEXT,
                global_webhook_id TEXT,
                global_webhook_token TEXT,
                created_at TEXT DEFAULT (CURRENT_TIMESTAMP),
                updated_at TEXT DEFAULT (CURRENT_TIMESTAMP)
            );
        "#;

        const USER_VOTE_SCHEMA: &str = r#"
            CREATE TABLE IF NOT EXISTS user_vote (
                id TEXT PRIMARY KEY,
                user_id TEXT NOT NULL,
                voted_at TEXT NOT NULL,
                expires_at TEXT NOT NULL,
                vote_type TEXT NOT NULL DEFAULT 'vote'
            );
        "#;

        conn.execute_batch(GUILD_SCHEMA).await?;
        conn.execute_batch(USER_VOTE_SCHEMA).await?;

        Ok(())
    }

    fn now_iso() -> String {
        Utc::now().to_rfc3339()
    }

    pub async fn get_locale(&self, guild_id: &str) -> anyhow::Result<String> {
        if let Some(locale) = self
            .cache
            .read()
            .await
            .get(guild_id)
            .and_then(|c| c.locale.clone())
        {
            return Ok(locale);
        }

        let conn = self.db.connect()?;
        let mut rows = conn
            .query(
                "SELECT locale FROM guild WHERE id = ?1 LIMIT 1",
                params![guild_id],
            )
            .await?;

        if let Some(row) = rows.next().await? {
            let locale: Option<String> = row.get(0)?;
            if let Some(locale) = locale {
                self.cache
                    .write()
                    .await
                    .entry(guild_id.to_string())
                    .or_insert_with(Default::default)
                    .locale = Some(locale.clone());
                return Ok(locale);
            }
        }

        Ok(DEFAULT_LOCALE.to_string())
    }

    pub async fn get_prefix(&self, guild_id: &str) -> anyhow::Result<String> {
        if let Some(prefix) = self
            .cache
            .read()
            .await
            .get(guild_id)
            .and_then(|c| c.prefix.clone())
        {
            return Ok(prefix);
        }

        let conn = self.db.connect()?;
        let mut rows = conn
            .query(
                "SELECT prefix FROM guild WHERE id = ?1 LIMIT 1",
                params![guild_id],
            )
            .await?;

        if let Some(row) = rows.next().await? {
            let prefix: Option<String> = row.get(0)?;
            if let Some(prefix) = prefix {
                self.cache
                    .write()
                    .await
                    .entry(guild_id.to_string())
                    .or_insert_with(Default::default)
                    .prefix = Some(prefix.clone());
                return Ok(prefix);
            }
        }

        Ok(DEFAULT_PREFIX.to_string())
    }

    pub async fn set_locale(&self, guild_id: &str, locale: &str) -> anyhow::Result<()> {
        let sql = "INSERT INTO guild (id, locale, created_at, updated_at) VALUES (?1, ?2, CURRENT_TIMESTAMP, CURRENT_TIMESTAMP)
            ON CONFLICT(id) DO UPDATE SET locale = excluded.locale, updated_at = CURRENT_TIMESTAMP";

        let conn = self.db.connect()?;
        conn.execute(sql, params![guild_id, locale]).await?;

        self.cache
            .write()
            .await
            .entry(guild_id.to_string())
            .or_insert_with(Default::default)
            .locale = Some(locale.to_string());

        Ok(())
    }

    pub async fn set_prefix(&self, guild_id: &str, prefix: &str) -> anyhow::Result<()> {
        let sql = "INSERT INTO guild (id, prefix, created_at, updated_at) VALUES (?1, ?2, CURRENT_TIMESTAMP, CURRENT_TIMESTAMP)
            ON CONFLICT(id) DO UPDATE SET prefix = excluded.prefix, updated_at = CURRENT_TIMESTAMP";

        let conn = self.db.connect()?;
        conn.execute(sql, params![guild_id, prefix]).await?;

        self.cache
            .write()
            .await
            .entry(guild_id.to_string())
            .or_insert_with(Default::default)
            .prefix = Some(prefix.to_string());

        Ok(())
    }

    pub async fn delete_prefix(&self, guild_id: &str) -> anyhow::Result<()> {
        let sql = "INSERT INTO guild (id, prefix, created_at, updated_at) VALUES (?1, NULL, CURRENT_TIMESTAMP, CURRENT_TIMESTAMP)
            ON CONFLICT(id) DO UPDATE SET prefix = NULL, updated_at = CURRENT_TIMESTAMP";

        let conn = self.db.connect()?;
        conn.execute(sql, params![guild_id]).await?;

        if let Some(cached) = self.cache.write().await.get_mut(guild_id) {
            cached.prefix = None;
        }

        Ok(())
    }

    pub async fn get_chatbot_setup(&self, guild_id: &str) -> anyhow::Result<Option<ISetup>> {
        let conn = self.db.connect()?;
        let mut rows = conn
            .query(
                "SELECT id, chatbot_channel_id, created_at FROM guild WHERE id = ?1 LIMIT 1",
                params![guild_id],
            )
            .await?;

        if let Some(row) = rows.next().await? {
            let id: String = row.get(0)?;
            let channel_id: Option<String> = row.get(1)?;
            let created_at: Option<String> = row.get(2)?;

            if let Some(channel_id) = channel_id {
                let created_at = created_at
                    .and_then(|c| DateTime::parse_from_rfc3339(&c).ok())
                    .map(|d| d.with_timezone(&Utc))
                    .unwrap_or_else(Utc::now);

                return Ok(Some(ISetup {
                    id: id.clone(),
                    guild_id: id,
                    channel_id,
                    created_at,
                }));
            }
        }

        Ok(None)
    }

    pub async fn create_chatbot_setup(
        &self,
        guild_id: &str,
        channel_id: &str,
    ) -> anyhow::Result<()> {
        let sql = "INSERT INTO guild (id, chatbot_channel_id, created_at, updated_at) VALUES (?1, ?2, CURRENT_TIMESTAMP, CURRENT_TIMESTAMP)
            ON CONFLICT(id) DO UPDATE SET chatbot_channel_id = excluded.chatbot_channel_id, updated_at = CURRENT_TIMESTAMP";

        let conn = self.db.connect()?;
        conn.execute(sql, params![guild_id, channel_id]).await?;

        Ok(())
    }

    pub async fn delete_chatbot_setup(&self, guild_id: &str) -> anyhow::Result<()> {
        let sql = "UPDATE guild SET chatbot_channel_id = NULL, updated_at = CURRENT_TIMESTAMP WHERE id = ?1";

        let conn = self.db.connect()?;
        conn.execute(sql, params![guild_id]).await?;

        Ok(())
    }

    pub async fn get_global_chat_channel(&self, guild_id: &str) -> anyhow::Result<Option<String>> {
        let conn = self.db.connect()?;
        let mut rows = conn
            .query(
                "SELECT global_channel_id FROM guild WHERE id = ?1 LIMIT 1",
                params![guild_id],
            )
            .await?;

        if let Some(row) = rows.next().await? {
            let channel: Option<String> = row.get(0)?;
            return Ok(channel);
        }

        Ok(None)
    }

    pub async fn create_global_chat_channel(
        &self,
        guild_id: &str,
        channel_id: &str,
        webhook_id: Option<&str>,
        webhook_token: Option<&str>,
    ) -> anyhow::Result<()> {
        let sql = "INSERT INTO guild (id, global_channel_id, global_webhook_id, global_webhook_token, created_at, updated_at) VALUES (?1, ?2, ?3, ?4, CURRENT_TIMESTAMP, CURRENT_TIMESTAMP)
            ON CONFLICT(id) DO UPDATE SET global_channel_id = excluded.global_channel_id, global_webhook_id = excluded.global_webhook_id, global_webhook_token = excluded.global_webhook_token, updated_at = CURRENT_TIMESTAMP";

        let conn = self.db.connect()?;
        conn.execute(
            sql,
            params![guild_id, channel_id, webhook_id, webhook_token],
        )
        .await?;

        Ok(())
    }

    pub async fn delete_global_chat_channel(&self, guild_id: &str) -> anyhow::Result<()> {
        let sql = "UPDATE guild SET global_channel_id = NULL, global_webhook_id = NULL, global_webhook_token = NULL, updated_at = CURRENT_TIMESTAMP WHERE id = ?1";

        let conn = self.db.connect()?;
        conn.execute(sql, params![guild_id]).await?;

        Ok(())
    }

    pub async fn get_all_global_chat(
        &self,
    ) -> anyhow::Result<Vec<(String, String, Option<String>, Option<String>)>> {
        let conn = self.db.connect()?;
        let stmt = conn
            .prepare(
                "SELECT id, global_channel_id, global_webhook_id, global_webhook_token FROM guild WHERE global_channel_id IS NOT NULL",
            )
            .await?;
        let mut rows = stmt.query(()).await?;

        let mut result = Vec::new();
        while let Some(row) = rows.next().await? {
            let id: String = row.get(0)?;
            let channel_id: String = row.get(1)?;
            let webhook_id: Option<String> = row.get(2)?;
            let webhook_token: Option<String> = row.get(3)?;
            result.push((id, channel_id, webhook_id, webhook_token));
        }

        Ok(result)
    }

    pub async fn get_premium_status(&self, user_id: &str) -> anyhow::Result<Option<PremiumStatus>> {
        let now_iso = Self::now_iso();

        let vote = self.fetch_latest_vote(user_id, "regular", &now_iso).await?;

        if let Some(v) = vote {
            let expires = DateTime::parse_from_rfc3339(&v.0)?.with_timezone(&Utc);
            return Ok(Some(PremiumStatus {
                vote_type: "regular".to_string(),
                time_remaining: expires.timestamp().saturating_sub(Utc::now().timestamp()) as u64,
            }));
        }

        let vote = self.fetch_latest_vote(user_id, "vote", &now_iso).await?;

        if let Some(v) = vote {
            let expires = DateTime::parse_from_rfc3339(&v.0)?.with_timezone(&Utc);
            return Ok(Some(PremiumStatus {
                vote_type: "vote".to_string(),
                time_remaining: expires.timestamp().saturating_sub(Utc::now().timestamp()) as u64,
            }));
        }

        Ok(None)
    }

    pub async fn add_user_vote(&self, user_id: &str) -> anyhow::Result<()> {
        let expires_at = Utc::now() + Duration::hours(VOTE_PREMIUM_DURATION_HOURS);
        self.insert_vote(user_id, "vote", expires_at).await
    }

    pub async fn add_regular_premium(
        &self,
        user_id: &str,
        duration_secs: u64,
    ) -> anyhow::Result<()> {
        let expires_at = Utc::now() + Duration::seconds(duration_secs as i64);
        self.insert_vote(user_id, "regular", expires_at).await
    }

    pub async fn add_premium(
        &self,
        user_id: &str,
        premium_type: &str,
        duration_secs: Option<u64>,
    ) -> anyhow::Result<()> {
        let expires_at = if premium_type == "regular" {
            let secs = duration_secs
                .ok_or_else(|| anyhow::anyhow!("duration_secs required for regular premium"))?;
            Utc::now() + Duration::seconds(secs as i64)
        } else {
            Utc::now() + Duration::hours(VOTE_PREMIUM_DURATION_HOURS)
        };

        self.insert_vote(user_id, premium_type, expires_at).await
    }

    pub async fn has_active_premium(&self, user_id: &str) -> anyhow::Result<bool> {
        let now_iso = Self::now_iso();
        let conn = self.db.connect()?;
        let stmt = conn
            .prepare(
                "SELECT 1 FROM user_vote WHERE user_id = ?1 AND datetime(expires_at) > datetime(?2) LIMIT 1",
            )
            .await?;
        let mut rows = stmt.query(params![user_id, now_iso]).await?;

        Ok(rows.next().await?.is_some())
    }

    pub async fn get_premium_time_remaining(&self, user_id: &str) -> anyhow::Result<Option<u64>> {
        let now_iso = Self::now_iso();
        let conn = self.db.connect()?;
        let stmt = conn
            .prepare(
                "SELECT expires_at FROM user_vote WHERE user_id = ?1 AND datetime(expires_at) > datetime(?2) ORDER BY datetime(expires_at) DESC LIMIT 1",
            )
            .await?;
        let mut rows = stmt.query(params![user_id, now_iso]).await?;

        if let Some(row) = rows.next().await? {
            let expires_at: String = row.get(0)?;
            let expires = DateTime::parse_from_rfc3339(&expires_at)?.with_timezone(&Utc);
            return Ok(Some(
                expires.timestamp().saturating_sub(Utc::now().timestamp()) as u64,
            ));
        }

        Ok(None)
    }

    pub async fn clear_vote_data(&self, user_id: &str) -> anyhow::Result<()> {
        let conn = self.db.connect()?;
        conn.execute("DELETE FROM user_vote WHERE user_id = ?1", params![user_id])
            .await?;

        Ok(())
    }

    pub async fn clear_premium_data(&self, user_id: &str) -> anyhow::Result<()> {
        let conn = self.db.connect()?;
        conn.execute(
            "DELETE FROM user_vote WHERE user_id = ?1 AND vote_type = 'regular'",
            params![user_id],
        )
        .await?;

        Ok(())
    }

    pub async fn cleanup_expired_votes(&self) -> anyhow::Result<u64> {
        let now_iso = Self::now_iso();
        let conn = self.db.connect()?;
        let result = conn
            .execute(
                "DELETE FROM user_vote WHERE datetime(expires_at) <= datetime(?1)",
                params![now_iso],
            )
            .await?;

        Ok(result)
    }

    pub async fn get_vote_stats(&self, user_id: Option<&str>) -> anyhow::Result<VoteStats> {
        let conn = self.db.connect()?;

        let (total_sql, active_sql) = if user_id.is_some() {
            (
                "SELECT COUNT(*) FROM user_vote WHERE user_id = ?1",
                "SELECT COUNT(*) FROM user_vote WHERE user_id = ?1 AND datetime(expires_at) > datetime(?2)",
            )
        } else {
            (
                "SELECT COUNT(*) FROM user_vote",
                "SELECT COUNT(*) FROM user_vote WHERE datetime(expires_at) > datetime(?1)",
            )
        };

        let total = if let Some(uid) = user_id {
            let mut rows = conn.query(total_sql, params![uid]).await?;
            rows.next()
                .await?
                .map(|r| r.get::<i64>(0))
                .transpose()?
                .unwrap_or(0)
        } else {
            let mut rows = conn.query(total_sql, ()).await?;
            rows.next()
                .await?
                .map(|r| r.get::<i64>(0))
                .transpose()?
                .unwrap_or(0)
        };

        let active = if let Some(uid) = user_id {
            let mut rows = conn
                .query(active_sql, params![uid, Self::now_iso()])
                .await?;
            rows.next()
                .await?
                .map(|r| r.get::<i64>(0))
                .transpose()?
                .unwrap_or(0)
        } else {
            let mut rows = conn.query(active_sql, params![Self::now_iso()]).await?;
            rows.next()
                .await?
                .map(|r| r.get::<i64>(0))
                .transpose()?
                .unwrap_or(0)
        };

        Ok(VoteStats { total, active })
    }

    pub async fn get_premium_stats(&self, user_id: Option<&str>) -> anyhow::Result<PremiumStats> {
        let now_iso = Self::now_iso();

        if let Some(uid) = user_id {
            let conn = self.db.connect()?;
            let mut rows = conn
                .query(
                    "SELECT vote_type, expires_at FROM user_vote WHERE user_id = ?1 AND datetime(expires_at) > datetime(?2) ORDER BY datetime(expires_at) DESC LIMIT 1",
                    params![uid, now_iso],
                )
                .await?;

            if let Some(row) = rows.next().await? {
                let vote_type: String = row.get(0)?;
                let expires_at: String = row.get(1)?;
                let expires = DateTime::parse_from_rfc3339(&expires_at)?.with_timezone(&Utc);
                return Ok(PremiumStats {
                    active: true,
                    expires_at: Some(expires),
                    vote_type: Some(vote_type),
                });
            }

            return Ok(PremiumStats {
                active: false,
                expires_at: None,
                vote_type: None,
            });
        }

        let conn = self.db.connect()?;
        let mut rows = conn
            .query(
                "SELECT COUNT(*) FROM user_vote WHERE vote_type = 'regular' AND datetime(expires_at) > datetime(?1)",
                params![now_iso],
            )
            .await?;
        let regular_count = rows
            .next()
            .await?
            .map(|r| r.get::<i64>(0))
            .transpose()?
            .unwrap_or(0);

        let mut rows = conn
            .query(
                "SELECT COUNT(*) FROM user_vote WHERE vote_type = 'vote' AND datetime(expires_at) > datetime(?1)",
                params![Self::now_iso()],
            )
            .await?;
        let vote_count = rows
            .next()
            .await?
            .map(|r| r.get::<i64>(0))
            .transpose()?
            .unwrap_or(0);

        Ok(PremiumStats {
            active: regular_count > 0 || vote_count > 0,
            expires_at: None,
            vote_type: None,
        })
    }

    async fn insert_vote(
        &self,
        user_id: &str,
        vote_type: &str,
        expires_at: DateTime<Utc>,
    ) -> anyhow::Result<()> {
        let id = Uuid::new_v4().to_string();
        let voted_at = Utc::now().to_rfc3339();
        let expires_iso = expires_at.to_rfc3339();

        let conn = self.db.connect()?;
        conn.execute(
            "INSERT INTO user_vote (id, user_id, voted_at, expires_at, vote_type) VALUES (?1, ?2, ?3, ?4, ?5)",
            params![id, user_id, voted_at, expires_iso, vote_type],
        )
        .await?;

        Ok(())
    }

    async fn fetch_latest_vote(
        &self,
        user_id: &str,
        vote_type: &str,
        now_iso: &str,
    ) -> anyhow::Result<Option<(String, String)>> {
        let conn = self.db.connect()?;
        let mut rows = conn
            .query(
                "SELECT expires_at, vote_type FROM user_vote WHERE user_id = ?1 AND vote_type = ?2 AND datetime(expires_at) > datetime(?3) ORDER BY datetime(expires_at) DESC LIMIT 1",
                params![user_id, vote_type, now_iso],
            )
            .await?;

        if let Some(row) = rows.next().await? {
            let expires_at: String = row.get(0)?;
            let vote_type: String = row.get(1)?;
            return Ok(Some((expires_at, vote_type)));
        }

        Ok(None)
    }
}
