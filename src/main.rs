mod commands;
mod components;
mod config;
mod database;
mod events;
mod handlers;
mod types;
mod utils;

use anyhow::Result;
use std::env;
use std::sync::Arc;

use twilight_cache_inmemory::{InMemoryCache, ResourceType};
use twilight_gateway::{Event, EventTypeFlags, Shard, ShardId, StreamExt};
use twilight_http::Client as HttpClient;
use twilight_standby::Standby;

use crate::config::Config;
use crate::database::hybrid::HybridStore;
use crate::database::service::AlyaDatabase;
use handlers::{get_default_intents, HandlersSetup};
use types::BotContext;
use types::SlashCommand;
use types::SlashCommandContext;
use utils::init_logger;

#[tokio::main]
async fn main() -> Result<()> {
    dotenv::dotenv().ok();

    init_logger();

    let _ = rustls::crypto::aws_lc_rs::default_provider().install_default();

    tracing::info!("Starting Alya-chan Discord Bot...");

    let token = env::var("DISCORD_TOKEN").expect("DISCORD_TOKEN environment variable is required");

    let http = Arc::new(HttpClient::new(token.clone()));

    let cache = Arc::new(
        InMemoryCache::builder()
            .resource_types(ResourceType::MESSAGE | ResourceType::USER | ResourceType::GUILD)
            .build(),
    );

    let standby = Arc::new(Standby::new());

    let config = Arc::new(
        Config::load_with_overrides("./config.toml").expect("Missing or invalid ./config.toml — please create a valid config.toml in the project root. See README for examples."),
    );

    // Initialize hybrid database (local SQLite + optional Turso/libsql)
    let turso_url = env::var("TURSO_URL").ok();
    let turso_token = env::var("TURSO_TOKEN").ok();
    if let Err(e) =
        AlyaDatabase::init("data/alya.db", turso_url.as_deref(), turso_token.as_deref()).await
    {
        tracing::error!("Failed to initialize AlyaDatabase: {}", e);
    }
    if let Err(e) =
        HybridStore::init("data/alya.db", turso_url.as_deref(), turso_token.as_deref()).await
    {
        tracing::error!("Failed to initialize hybrid store: {}", e);
    }

    let bot_context = BotContext::new(
        Arc::clone(&http),
        Arc::clone(&cache),
        Arc::clone(&standby),
        Arc::clone(&config),
    );

    let handlers = HandlersSetup::new();

    let cmd_mgr = Arc::clone(&handlers.command_manager);
    let help_cmd_clone = Arc::clone(&handlers.help_command);
    let event_manager = Arc::clone(&handlers.event_manager);
    let component_manager = Arc::clone(&handlers.component_manager);

    let intents = get_default_intents();

    let mut shard = Shard::new(ShardId::ONE, token, intents);

    tracing::info!("Bot setup complete, connecting to Discord...");
    tracing::info!("Registered commands: {}", cmd_mgr.get_all_commands().len());

    loop {
        let event = match shard.next_event(EventTypeFlags::all()).await {
            Some(Ok(event)) => event,
            Some(Err(source)) => {
                tracing::warn!(?source, "error receiving event");
                // is_fatal() doesn't exist in 0.17, so we break on any error
                break;
            }
            None => break,
        };

        cache.update(&event);

        standby.process(&event);

        let bot = bot_context.clone();
        let cmd_mgr_clone = Arc::clone(&cmd_mgr);
        let help_cmd_clone = Arc::clone(&help_cmd_clone);
        let evt_mgr = Arc::clone(&event_manager);
        let comp_mgr = Arc::clone(&component_manager);

        tokio::spawn(async move {
            if let Err(e) = evt_mgr.process_event(bot.clone(), event.clone()).await {
                tracing::error!("Error processing event: {}", e);
            }

            if let Event::InteractionCreate(interaction) = &event {
                if let Some(data) = &interaction.data {
                    match data {
                        twilight_model::application::interaction::InteractionData::ApplicationCommand(cmd_data) => {
                            let cmd_name = &cmd_data.name;
                            let interaction_id = interaction.id;
                            let application_id = interaction.application_id;
                            let author_id = interaction.author_id();
                            let guild_id = interaction.guild_id;
                            let token = interaction.token.clone();

                            tracing::info!("Received slash command: {}", cmd_name);

                            if cmd_name == "help" {
                                let help_as_cmd: Arc<dyn SlashCommand> = help_cmd_clone.clone();
                                if let Err(e) = help_as_cmd.execute(
                                    &SlashCommandContext::new(
                                        bot.clone(),
                                        interaction_id,
                                        application_id,
                                        author_id,
                                        guild_id,
                                        token,
                                        (**cmd_data).clone(),
                                    )
                                ).await {
                                    tracing::error!("Error executing help command: {}", e);
                                }
                            } else if let Some(command) = cmd_mgr_clone.get(cmd_name) {
                                let ctx = SlashCommandContext::new(
                                    bot.clone(),
                                    interaction_id,
                                    application_id,
                                    author_id,
                                    guild_id,
                                    token,
                                    (**cmd_data).clone(),
                                );
                                if let Err(e) = command.execute(&ctx).await {
                                    tracing::error!("Error executing command '{}': {}", cmd_name, e);
                                }
                            } else {
                                tracing::warn!("Command not found: {}", cmd_name);
                            }
                        }
                        twilight_model::application::interaction::InteractionData::MessageComponent(_) => {
                            let interaction_inner = &interaction.0;
                            if let Err(e) = comp_mgr.process_interaction(bot.clone(), interaction_inner.clone()).await {
                                tracing::error!("Error processing interaction: {}", e);
                            }
                        }
                        _ => {}
                    }
                }
            }
        });
    }

    Ok(())
}
