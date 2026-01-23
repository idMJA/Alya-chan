use crate::database::schema::{guild, user_vote};
use chrono::{Duration, Utc};
use sea_orm::*;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use uuid::Uuid;

const DEFAULT_LOCALE: &str = "en-US";
const DEFAULT_PREFIX: &str = "!";
const VOTE_PREMIUM_DURATION_HOURS: i64 = 12;

/// Guild setup response type
pub struct ISetup {
    pub id: String,
    pub guild_id: String,
    pub channel_id: String,
    pub created_at: chrono::DateTime<Utc>,
}

/// Premium status response
pub struct PremiumStatus {
    pub vote_type: String,
    pub time_remaining: u64,
}

/// Vote statistics response
pub struct VoteStats {
    pub total: i64,
    pub active: i64,
}

/// Premium statistics response
#[derive(Clone)]
pub struct PremiumStats {
    pub active: bool,
    pub expires_at: Option<chrono::DateTime<Utc>>,
    pub vote_type: Option<String>,
}

/// Database service for Alya-chan
pub struct AlyaDatabase {
    pub db: DbConn,
    cache: Arc<RwLock<HashMap<String, CachedGuild>>>,
}

#[derive(Clone)]
struct CachedGuild {
    locale: Option<String>,
    prefix: Option<String>,
}

impl AlyaDatabase {
    /// Create a new database service
    pub fn new(db: DbConn) -> Self {
        Self {
            db,
            cache: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    // ===== Guild Queries =====

    /// Get the locale for a guild from the database, or return default if not found
    pub async fn get_locale(&self, guild_id: &str) -> anyhow::Result<String> {
        let cache = self.cache.read().await;
        if let Some(cached) = cache.get(guild_id) {
            if let Some(locale) = &cached.locale {
                return Ok(locale.clone());
            }
        }
        drop(cache);

        let guild = guild::Entity::find_by_id(guild_id)
            .one(&self.db)
            .await?;

        if let Some(model) = guild {
            if let Some(locale) = &model.locale {
                let mut cache = self.cache.write().await;
                cache.entry(guild_id.to_string()).or_insert_with(|| {
                    CachedGuild {
                        locale: Some(locale.clone()),
                        prefix: None,
                    }
                });
                return Ok(locale.clone());
            }
        }

        Ok(DEFAULT_LOCALE.to_string())
    }

    /// Get the prefix for a guild from the database, or return default if not found
    pub async fn get_prefix(&self, guild_id: &str) -> anyhow::Result<String> {
        let cache = self.cache.read().await;
        if let Some(cached) = cache.get(guild_id) {
            if let Some(prefix) = &cached.prefix {
                return Ok(prefix.clone());
            }
        }
        drop(cache);

        let guild = guild::Entity::find_by_id(guild_id)
            .one(&self.db)
            .await?;

        if let Some(model) = guild {
            if let Some(prefix) = &model.prefix {
                return Ok(prefix.clone());
            }
        }

        Ok(DEFAULT_PREFIX.to_string())
    }

    /// Set the locale for a guild
    pub async fn set_locale(&self, guild_id: &str, locale: &str) -> anyhow::Result<()> {
        let existing = guild::Entity::find_by_id(guild_id)
            .one(&self.db)
            .await?;

        if existing.is_some() {
            guild::Entity::update_many()
                .set(guild::ActiveModel {
                    locale: ActiveValue::Set(Some(locale.to_string())),
                    ..Default::default()
                })
                .filter(guild::Column::Id.eq(guild_id))
                .exec(&self.db)
                .await?;
        } else {
            let model = guild::ActiveModel {
                id: ActiveValue::Set(guild_id.to_string()),
                locale: ActiveValue::Set(Some(locale.to_string())),
                created_at: ActiveValue::Set(Utc::now()),
                updated_at: ActiveValue::Set(Utc::now()),
                ..Default::default()
            };
            guild::Entity::insert(model).exec(&self.db).await?;
        }

        // Invalidate cache
        let mut cache = self.cache.write().await;
        if let Some(cached) = cache.get_mut(guild_id) {
            cached.locale = Some(locale.to_string());
        }

        Ok(())
    }

    /// Set the prefix for a guild
    pub async fn set_prefix(&self, guild_id: &str, prefix: &str) -> anyhow::Result<()> {
        let existing = guild::Entity::find_by_id(guild_id)
            .one(&self.db)
            .await?;

        if existing.is_some() {
            guild::Entity::update_many()
                .set(guild::ActiveModel {
                    prefix: ActiveValue::Set(Some(prefix.to_string())),
                    ..Default::default()
                })
                .filter(guild::Column::Id.eq(guild_id))
                .exec(&self.db)
                .await?;
        } else {
            let model = guild::ActiveModel {
                id: ActiveValue::Set(guild_id.to_string()),
                prefix: ActiveValue::Set(Some(prefix.to_string())),
                created_at: ActiveValue::Set(Utc::now()),
                updated_at: ActiveValue::Set(Utc::now()),
                ..Default::default()
            };
            guild::Entity::insert(model).exec(&self.db).await?;
        }

        // Invalidate cache
        let mut cache = self.cache.write().await;
        if let Some(cached) = cache.get_mut(guild_id) {
            cached.prefix = Some(prefix.to_string());
        }

        Ok(())
    }

    // ===== Chatbot Setup =====

    /// Get the chatbot setup for a guild
    pub async fn get_chatbot_setup(&self, guild_id: &str) -> anyhow::Result<Option<ISetup>> {
        let guild = guild::Entity::find_by_id(guild_id)
            .one(&self.db)
            .await?;

        Ok(guild.and_then(|model| {
            model.chatbot_channel_id.map(|channel_id| ISetup {
                id: model.id.clone(),
                guild_id: model.id,
                channel_id,
                created_at: model.created_at,
            })
        }))
    }

    /// Create or update chatbot setup for a guild
    pub async fn create_chatbot_setup(
        &self,
        guild_id: &str,
        channel_id: &str,
    ) -> anyhow::Result<()> {
        let existing = guild::Entity::find_by_id(guild_id)
            .one(&self.db)
            .await?;

        if existing.is_some() {
            guild::Entity::update_many()
                .set(guild::ActiveModel {
                    chatbot_channel_id: ActiveValue::Set(Some(channel_id.to_string())),
                    ..Default::default()
                })
                .filter(guild::Column::Id.eq(guild_id))
                .exec(&self.db)
                .await?;
        } else {
            let model = guild::ActiveModel {
                id: ActiveValue::Set(guild_id.to_string()),
                chatbot_channel_id: ActiveValue::Set(Some(channel_id.to_string())),
                created_at: ActiveValue::Set(Utc::now()),
                updated_at: ActiveValue::Set(Utc::now()),
                ..Default::default()
            };
            guild::Entity::insert(model).exec(&self.db).await?;
        }

        Ok(())
    }

    /// Delete chatbot setup for a guild
    pub async fn delete_chatbot_setup(&self, guild_id: &str) -> anyhow::Result<()> {
        guild::Entity::update_many()
            .set(guild::ActiveModel {
                chatbot_channel_id: ActiveValue::Set(None),
                ..Default::default()
            })
            .filter(guild::Column::Id.eq(guild_id))
            .exec(&self.db)
            .await?;

        Ok(())
    }

    // ===== Global Chat =====

    /// Get the global chat channel for a guild
    pub async fn get_global_chat_channel(&self, guild_id: &str) -> anyhow::Result<Option<String>> {
        let guild = guild::Entity::find_by_id(guild_id)
            .one(&self.db)
            .await?;

        Ok(guild.and_then(|model| model.global_channel_id))
    }

    /// Create or update global chat channel for a guild
    pub async fn create_global_chat_channel(
        &self,
        guild_id: &str,
        channel_id: &str,
        webhook_id: Option<&str>,
        webhook_token: Option<&str>,
    ) -> anyhow::Result<()> {
        let existing = guild::Entity::find_by_id(guild_id)
            .one(&self.db)
            .await?;

        if existing.is_some() {
            guild::Entity::update_many()
                .set(guild::ActiveModel {
                    global_channel_id: ActiveValue::Set(Some(channel_id.to_string())),
                    global_webhook_id: ActiveValue::Set(webhook_id.map(|s| s.to_string())),
                    global_webhook_token: ActiveValue::Set(webhook_token.map(|s| s.to_string())),
                    ..Default::default()
                })
                .filter(guild::Column::Id.eq(guild_id))
                .exec(&self.db)
                .await?;
        } else {
            let model = guild::ActiveModel {
                id: ActiveValue::Set(guild_id.to_string()),
                global_channel_id: ActiveValue::Set(Some(channel_id.to_string())),
                global_webhook_id: ActiveValue::Set(webhook_id.map(|s| s.to_string())),
                global_webhook_token: ActiveValue::Set(webhook_token.map(|s| s.to_string())),
                created_at: ActiveValue::Set(Utc::now()),
                updated_at: ActiveValue::Set(Utc::now()),
                ..Default::default()
            };
            guild::Entity::insert(model).exec(&self.db).await?;
        }

        Ok(())
    }

    /// Delete global chat channel for a guild
    pub async fn delete_global_chat_channel(&self, guild_id: &str) -> anyhow::Result<()> {
        guild::Entity::update_many()
            .set(guild::ActiveModel {
                global_channel_id: ActiveValue::Set(None),
                global_webhook_id: ActiveValue::Set(None),
                global_webhook_token: ActiveValue::Set(None),
                ..Default::default()
            })
            .filter(guild::Column::Id.eq(guild_id))
            .exec(&self.db)
            .await?;

        Ok(())
    }

    /// Get all guilds with global chat channels set
    pub async fn get_all_global_chat(
        &self,
    ) -> anyhow::Result<Vec<(String, String, Option<String>, Option<String>)>> {
        let guilds = guild::Entity::find()
            .filter(guild::Column::GlobalChannelId.is_not_null())
            .all(&self.db)
            .await?;

        Ok(guilds
            .into_iter()
            .filter_map(|g| {
                g.global_channel_id.map(|ch| {
                    (g.id, ch, g.global_webhook_id, g.global_webhook_token)
                })
            })
            .collect())
    }

    // ===== Premium & Votes =====

    /// Get premium status for a user
    pub async fn get_premium_status(&self, user_id: &str) -> anyhow::Result<Option<PremiumStatus>> {
        let now = Utc::now();

        // Check for regular premium first
        let vote = user_vote::Entity::find()
            .filter(user_vote::Column::UserId.eq(user_id))
            .filter(user_vote::Column::VoteType.eq("regular"))
            .filter(user_vote::Column::ExpiresAt.gt(now))
            .order_by_desc(user_vote::Column::ExpiresAt)
            .one(&self.db)
            .await?;

        if let Some(v) = vote {
            return Ok(Some(PremiumStatus {
                vote_type: v.vote_type,
                time_remaining: (v.expires_at.timestamp() as u64)
                    .saturating_sub(now.timestamp() as u64),
            }));
        }

        // Check for vote premium
        let vote = user_vote::Entity::find()
            .filter(user_vote::Column::UserId.eq(user_id))
            .filter(user_vote::Column::VoteType.eq("vote"))
            .filter(user_vote::Column::ExpiresAt.gt(now))
            .order_by_desc(user_vote::Column::ExpiresAt)
            .one(&self.db)
            .await?;

        Ok(vote.map(|v| PremiumStatus {
            vote_type: v.vote_type,
            time_remaining: (v.expires_at.timestamp() as u64)
                .saturating_sub(now.timestamp() as u64),
        }))
    }

    /// Add a user vote (12 hours duration)
    pub async fn add_user_vote(&self, user_id: &str) -> anyhow::Result<()> {
        let expires_at = Utc::now() + Duration::hours(VOTE_PREMIUM_DURATION_HOURS);

        let model = user_vote::ActiveModel {
            id: ActiveValue::Set(Uuid::new_v4().to_string()),
            user_id: ActiveValue::Set(user_id.to_string()),
            voted_at: ActiveValue::Set(Utc::now()),
            expires_at: ActiveValue::Set(expires_at),
            vote_type: ActiveValue::Set("vote".to_string()),
        };

        user_vote::Entity::insert(model).exec(&self.db).await?;
        Ok(())
    }

    /// Add regular premium for a user (with custom duration in seconds)
    pub async fn add_regular_premium(
        &self,
        user_id: &str,
        duration_secs: u64,
    ) -> anyhow::Result<()> {
        let expires_at = Utc::now() + Duration::seconds(duration_secs as i64);

        let model = user_vote::ActiveModel {
            id: ActiveValue::Set(Uuid::new_v4().to_string()),
            user_id: ActiveValue::Set(user_id.to_string()),
            voted_at: ActiveValue::Set(Utc::now()),
            expires_at: ActiveValue::Set(expires_at),
            vote_type: ActiveValue::Set("regular".to_string()),
        };

        user_vote::Entity::insert(model).exec(&self.db).await?;
        Ok(())
    }

    /// Add premium (either vote or regular type)
    pub async fn add_premium(
        &self,
        user_id: &str,
        premium_type: &str,
        duration_secs: Option<u64>,
    ) -> anyhow::Result<()> {
        let expires_at = if premium_type == "regular" {
            let secs = duration_secs.ok_or_else(|| {
                anyhow::anyhow!("duration_secs required for regular premium")
            })?;
            Utc::now() + Duration::seconds(secs as i64)
        } else {
            Utc::now() + Duration::hours(VOTE_PREMIUM_DURATION_HOURS)
        };

        let model = user_vote::ActiveModel {
            id: ActiveValue::Set(Uuid::new_v4().to_string()),
            user_id: ActiveValue::Set(user_id.to_string()),
            voted_at: ActiveValue::Set(Utc::now()),
            expires_at: ActiveValue::Set(expires_at),
            vote_type: ActiveValue::Set(premium_type.to_string()),
        };

        user_vote::Entity::insert(model).exec(&self.db).await?;
        Ok(())
    }

    /// Check if a user has active premium
    pub async fn has_active_premium(&self, user_id: &str) -> anyhow::Result<bool> {
        let now = Utc::now();
        let vote = user_vote::Entity::find()
            .filter(user_vote::Column::UserId.eq(user_id))
            .filter(user_vote::Column::ExpiresAt.gt(now))
            .one(&self.db)
            .await?;

        Ok(vote.is_some())
    }

    /// Get premium time remaining for a user
    pub async fn get_premium_time_remaining(&self, user_id: &str) -> anyhow::Result<Option<u64>> {
        let now = Utc::now();

        let vote = user_vote::Entity::find()
            .filter(user_vote::Column::UserId.eq(user_id))
            .filter(user_vote::Column::ExpiresAt.gt(now))
            .order_by_desc(user_vote::Column::ExpiresAt)
            .one(&self.db)
            .await?;

        Ok(vote.map(|v| {
            (v.expires_at.timestamp() as u64).saturating_sub(now.timestamp() as u64)
        }))
    }

    /// Clear vote data for a user
    pub async fn clear_vote_data(&self, user_id: &str) -> anyhow::Result<()> {
        user_vote::Entity::delete_many()
            .filter(user_vote::Column::UserId.eq(user_id))
            .exec(&self.db)
            .await?;
        Ok(())
    }

    /// Clear premium data for a user
    pub async fn clear_premium_data(&self, user_id: &str) -> anyhow::Result<()> {
        user_vote::Entity::delete_many()
            .filter(user_vote::Column::UserId.eq(user_id))
            .filter(user_vote::Column::VoteType.eq("regular"))
            .exec(&self.db)
            .await?;
        Ok(())
    }

    /// Cleanup expired premium entries
    pub async fn cleanup_expired_votes(&self) -> anyhow::Result<u64> {
        let now = Utc::now();
        let result = user_vote::Entity::delete_many()
            .filter(user_vote::Column::ExpiresAt.lt(now))
            .exec(&self.db)
            .await?;

        Ok(result.rows_affected)
    }

    /// Get vote statistics
    pub async fn get_vote_stats(&self, user_id: Option<&str>) -> anyhow::Result<VoteStats> {
        let query = if let Some(uid) = user_id {
            user_vote::Entity::find().filter(user_vote::Column::UserId.eq(uid))
        } else {
            user_vote::Entity::find()
        };

        let votes = query.all(&self.db).await?;
        let now = Utc::now();
        let active = votes.iter().filter(|v| v.expires_at > now).count();

        Ok(VoteStats {
            total: votes.len() as i64,
            active: active as i64,
        })
    }

    /// Get premium statistics
    pub async fn get_premium_stats(
        &self,
        user_id: Option<&str>,
    ) -> anyhow::Result<PremiumStats> {
        let now = Utc::now();

        if let Some(uid) = user_id {
            let vote = user_vote::Entity::find()
                .filter(user_vote::Column::UserId.eq(uid))
                .filter(user_vote::Column::ExpiresAt.gt(now))
                .order_by_desc(user_vote::Column::ExpiresAt)
                .one(&self.db)
                .await?;

            Ok(PremiumStats {
                active: vote.is_some(),
                expires_at: vote.as_ref().map(|v| v.expires_at),
                vote_type: vote.map(|v| v.vote_type),
            })
        } else {
            let regular_count = user_vote::Entity::find()
                .filter(user_vote::Column::VoteType.eq("regular"))
                .filter(user_vote::Column::ExpiresAt.gt(now))
                .all(&self.db)
                .await?
                .len();

            let vote_count = user_vote::Entity::find()
                .filter(user_vote::Column::VoteType.eq("vote"))
                .filter(user_vote::Column::ExpiresAt.gt(now))
                .all(&self.db)
                .await?
                .len();

            Ok(PremiumStats {
                active: regular_count > 0 || vote_count > 0,
                expires_at: None,
                vote_type: None,
            })
        }
    }
}
