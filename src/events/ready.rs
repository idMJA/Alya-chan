use crate::events::guild_create::GuildCreateHandler;
use crate::types::{BotResult, EventContext, EventHandler, PresenceUpdate};
use crate::utils::constants::BOT_VERSION;
use crate::utils::topgg::TopGgPoster;
use async_trait::async_trait;
use std::time::Duration;
use tokio::sync::broadcast;
use twilight_cache_inmemory::InMemoryCache;
use twilight_model::gateway::event::Event;
use twilight_model::gateway::presence::Status;

pub struct ReadyHandler;

impl ReadyHandler {
    async fn register_commands(
        &self,
        ctx: &EventContext,
        application_id: twilight_model::id::Id<twilight_model::id::marker::ApplicationMarker>,
    ) -> BotResult<()> {
        let commands = ctx.bot.command_manager.get_all_commands();

        tracing::info!(
            "Registering {} slash commands to Discord...",
            commands.len()
        );

        // Build all commands using CommandBuilder
        let mut built_commands = Vec::new();
        for command in commands {
            built_commands.push(command.build());
        }

        // Register all commands at once (idempotent)
        ctx.bot
            .http
            .interaction(application_id)
            .set_global_commands(&built_commands)
            .await?;

        tracing::info!("Slash command registration completed");
        Ok(())
    }

    async fn start_rotating_presence(
        &self,
        cache: std::sync::Arc<InMemoryCache>,
        presence_tx: broadcast::Sender<PresenceUpdate>,
    ) {
        let mut interval = tokio::time::interval(Duration::from_secs(30));
        let mut rotation = 0usize;

        loop {
            interval.tick().await;

            let guild_count = cache.stats().guilds();
            let user_count = cache.stats().users();

            let activity_name = match rotation % 3 {
                0 => format!("with {user_count} users"),
                1 => format!("in {guild_count} guilds"),
                _ => format!("v{BOT_VERSION}"),
            };

            let update = PresenceUpdate {
                activity_name: activity_name.clone(),
                status: Status::Idle,
            };

            if let Err(e) = presence_tx.send(update) {
                tracing::error!("Failed to send presence update: {}", e);
            } else {
                tracing::debug!("Presence: {}", activity_name);
            }

            rotation += 1;
        }
    }
}

#[async_trait]
impl EventHandler for ReadyHandler {
    fn name(&self) -> &'static str {
        "ready"
    }

    async fn handle(&self, ctx: &EventContext) -> BotResult<()> {
        if let Event::Ready(ready) = &ctx.event {
            let shard_info = ready.shard.map_or_else(
                || "[Shard 1/1]".to_string(),
                |shard_tuple| {
                    format!(
                        "[Shard {}/{}]",
                        shard_tuple.number() + 1,
                        ctx.bot.shard_count
                    )
                },
            );

            tracing::info!(
                "{} Bot is ready! Logged in as {}#{} [v{}]",
                shard_info,
                ready.user.name,
                ready.user.discriminator,
                BOT_VERSION
            );

            tracing::info!(
                "{} Connected to {} guilds | Total shards: {}",
                shard_info,
                ready.guilds.len(),
                ctx.bot.shard_count
            );

            let current_shard = ready.shard.map_or(0, twilight_gateway::ShardId::number);
            if current_shard == 0 {
                // Register all slash commands to Discord
                self.register_commands(ctx, ready.application.id).await?;

                // Start rotating presence task
                let handler = Self;
                let cache = ctx.bot.cache.clone();
                let presence_tx = ctx.bot.presence_tx.clone();
                tokio::spawn(async move {
                    handler.start_rotating_presence(cache, presence_tx).await;
                });

                tracing::info!("Presence rotation started (30s interval)");

                if let Some(top_gg_config) = &ctx.bot.config.top_gg {
                    if top_gg_config.enabled {
                        let poster = TopGgPoster::new(top_gg_config.token.clone());

                        let cache = ctx.bot.cache.clone();
                        let shard_count = ctx.bot.shard_count;

                        // Spawn auto-posting task
                        tokio::spawn(async move {
                            if let Err(e) = poster.start_auto_posting(cache, shard_count).await {
                                tracing::error!("[Top.gg] Failed to start auto poster: {}", e);
                            }
                        });

                        tracing::info!("[Top.gg] Auto poster initialized");
                    } else {
                        tracing::info!("[Top.gg] Auto poster disabled in config");
                    }
                }
            }

            tokio::spawn(GuildCreateHandler::startup_complete());
        }

        Ok(())
    }
}
