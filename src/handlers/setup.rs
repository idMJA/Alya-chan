use super::{CommandManager, ComponentManager, EventManager};
use crate::commands::{setup_commands, HelpCommand};
use crate::events::{GuildCreateHandler, MessageCreateHandler, ReadyHandler};
use std::sync::Arc;
use twilight_gateway::Intents;

/// Bot handlers setup
pub struct HandlersSetup {
    pub command_manager: Arc<CommandManager>,
    pub event_manager: Arc<EventManager>,
    pub component_manager: Arc<ComponentManager>,
    pub help_command: Arc<HelpCommand>,
}

impl HandlersSetup {
    /// Setup semua handlers
    pub fn new() -> Self {
        // Setup command manager
        let mut cmd_mgr = CommandManager::new();
        setup_commands(&mut cmd_mgr);
        let cmd_mgr = Arc::new(cmd_mgr);

        // Setup help command dengan manager reference
        let help_cmd = Arc::new(HelpCommand::new().with_manager(Arc::clone(&cmd_mgr)));

        // Setup event manager
        let mut evt_mgr = EventManager::new();
        evt_mgr.register(Arc::new(ReadyHandler));
        evt_mgr.register(Arc::new(MessageCreateHandler));
        evt_mgr.register(Arc::new(GuildCreateHandler));
        let evt_mgr = Arc::new(evt_mgr);

        // Setup component manager
        let comp_mgr = Arc::new(ComponentManager::new());

        Self {
            command_manager: cmd_mgr,
            event_manager: evt_mgr,
            component_manager: comp_mgr,
            help_command: help_cmd,
        }
    }
}

impl Default for HandlersSetup {
    fn default() -> Self {
        Self::new()
    }
}

/// Get default gateway intents untuk Discord bot
pub fn get_default_intents() -> Intents {
    Intents::GUILDS
        | Intents::GUILD_MESSAGES
        | Intents::GUILD_MESSAGE_REACTIONS
        | Intents::MESSAGE_CONTENT
        | Intents::GUILD_MEMBERS
        | Intents::DIRECT_MESSAGES
}
