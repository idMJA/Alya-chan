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
use twilight_gateway::{Config as GatewayConfig, Event, EventTypeFlags, Shard, ShardId, StreamExt};
use twilight_http::Client as HttpClient;
use twilight_standby::Standby;

use crate::commands::HelpCommand;
use crate::config::Config;
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

    // Initialize hybrid database (local SQLite replica + optional Turso remote)
    let db_config = config.database.as_ref();
    let local_path = db_config
        .map(|d| d.local_path.as_str())
        .unwrap_or("data/alya.db");

    // Create data directory if it doesn't exist
    if let Some(parent) = std::path::Path::new(local_path).parent() {
        if !parent.as_os_str().is_empty() {
            if let Err(e) = std::fs::create_dir_all(parent) {
                tracing::error!("Failed to create data directory: {}", e);
                panic!("Cannot create data directory");
            }
        }
    }

    let remote_url = db_config.and_then(|d| d.remote_url.as_deref());
    let remote_token = db_config.and_then(|d| d.remote_token.as_deref());

    let db = match AlyaDatabase::init(local_path, remote_url, remote_token).await {
        Ok(db) => {
            // Sync local replica from remote on startup
            if let Err(e) = db.sync().await {
                tracing::warn!("Failed to sync database on startup: {}", e);
            }
            db
        }
        Err(e) => {
            tracing::error!("Failed to initialize AlyaDatabase: {}", e);
            panic!("Cannot start bot without database");
        }
    };

    // Get recommended shard count from Discord
    let gateway_info = http
        .gateway()
        .authed()
        .await
        .expect("Failed to get gateway info");

    let shard_count = gateway_info.model().await?.shards;

    tracing::info!(
        "Initializing {} shard(s) as recommended by Discord",
        shard_count
    );

    let handlers = HandlersSetup::new();

    let cmd_mgr = Arc::clone(&handlers.command_manager);
    let help_cmd_clone: Arc<HelpCommand> = Arc::clone(&handlers.help_command);
    let event_manager = Arc::clone(&handlers.event_manager);
    let component_manager = Arc::clone(&handlers.component_manager);

    let bot_context = BotContext::new(
        Arc::clone(&http),
        Arc::clone(&cache),
        Arc::clone(&standby),
        Arc::clone(&config),
        db,
        Arc::clone(&cmd_mgr),
        shard_count,
    );

    let intents = get_default_intents();

    // Create shards with automatic sharding
    let gateway_config = GatewayConfig::new(token.clone(), intents);

    let shards: Vec<Shard> = (0..shard_count)
        .map(|id| Shard::with_config(ShardId::new(id, shard_count), gateway_config.clone()))
        .collect();

    tracing::info!("Bot setup complete, connecting to Discord...");
    tracing::info!("Registered commands: {}", cmd_mgr.get_all_commands().len());

    // Spawn a task for each shard
    let mut tasks = Vec::new();

    for mut shard in shards {
        let shard_id = shard.id().number();
        let cache = Arc::clone(&cache);
        let standby = Arc::clone(&standby);
        let bot_context = bot_context.clone();
        let cmd_mgr = Arc::clone(&cmd_mgr);
        let help_cmd = Arc::clone(&help_cmd_clone);
        let event_manager = Arc::clone(&event_manager);
        let component_manager = Arc::clone(&component_manager);

        let task = tokio::spawn(async move {
            tracing::info!("Shard {} starting...", shard_id);

            loop {
                let event = match shard.next_event(EventTypeFlags::all()).await {
                    Some(Ok(event)) => event,
                    Some(Err(source)) => {
                        tracing::warn!(?source, "Shard {} error receiving event", shard_id);
                        continue;
                    }
                    None => {
                        tracing::warn!("Shard {} connection closed", shard_id);
                        break;
                    }
                };

                cache.update(&event);
                standby.process(&event);

                let bot = bot_context.clone();
                let cmd_mgr_clone = Arc::clone(&cmd_mgr);
                let help_cmd_clone = Arc::clone(&help_cmd);
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

            tracing::info!("Shard {} stopped", shard_id);
        });

        tasks.push(task);
    }

    // Wait for all shard tasks to complete
    for task in tasks {
        if let Err(e) = task.await {
            tracing::error!("Shard task error: {}", e);
        }
    }

    Ok(())
}
