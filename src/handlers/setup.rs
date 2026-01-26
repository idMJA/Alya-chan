use super::{CommandManager, ComponentManager, EventManager};
use crate::commands::{setup_commands, AboutButton, HelpCommand};
use crate::components::buttons::{GlobalChatCreateButton, SetupDeleteButton};
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
        cmd_mgr.log_summary();
        let cmd_mgr = Arc::new(cmd_mgr);

        // Setup help command dengan manager reference
        let help_cmd = Arc::new(HelpCommand::new().with_manager(Arc::clone(&cmd_mgr)));

        // Setup event manager
        let mut evt_mgr = EventManager::new();
        evt_mgr.register(Arc::new(ReadyHandler));
        evt_mgr.register(Arc::new(MessageCreateHandler));
        evt_mgr.register(Arc::new(GuildCreateHandler));
        evt_mgr.log_summary();
        let evt_mgr = Arc::new(evt_mgr);

        // Setup component manager
        let mut comp_manager = ComponentManager::new();

        // Register component handlers - AboutButton now from commands module
        comp_manager.register(Arc::new(AboutButton));
        comp_manager.register(Arc::new(SetupDeleteButton));
        comp_manager.register(Arc::new(GlobalChatCreateButton));
        comp_manager.register(Arc::new(
            crate::components::select_menus::help_menu::HelpMenuSelect::new(Arc::clone(&cmd_mgr)),
        ));
        comp_manager.register(Arc::new(
            crate::components::buttons::paginator::PaginatorButton::new(Arc::clone(&cmd_mgr)),
        ));
        comp_manager.log_summary();

        let comp_mgr = Arc::new(comp_manager);

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
