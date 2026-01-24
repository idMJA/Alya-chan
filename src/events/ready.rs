use crate::events::guild_create::GuildCreateHandler;
use crate::types::{BotResult, EventContext, EventHandler};
use crate::utils::constants::BOT_VERSION;
use crate::utils::topgg::TopGgPoster;
use async_trait::async_trait;
use twilight_model::application::command::CommandType;
use twilight_model::gateway::event::Event;
use twilight_util::builder::command::CommandBuilder;

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
            let command_data = CommandBuilder::new(
                command.name(),
                command.description(),
                CommandType::ChatInput,
            )
            .build();
            built_commands.push(command_data);
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
}

#[async_trait]
impl EventHandler for ReadyHandler {
    fn name(&self) -> &str {
        "ready"
    }

    async fn handle(&self, ctx: &EventContext) -> BotResult<()> {
        if let Event::Ready(ready) = &ctx.event {
            let shard_info = if let Some(shard_tuple) = ready.shard {
                format!(
                    "[Shard {}/{}]",
                    shard_tuple.number() + 1,
                    ctx.bot.shard_count
                )
            } else {
                "[Shard 1/1]".to_string()
            };

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

            let current_shard = ready.shard.map(|s| s.number()).unwrap_or(0);
            if current_shard == 0 {
                // Register all slash commands to Discord
                self.register_commands(ctx, ready.application.id).await?;

                if let Some(top_gg_config) = &ctx.bot.config.top_gg {
                    if top_gg_config.enabled {
                        let poster = TopGgPoster::new(top_gg_config.token.clone())?;

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
