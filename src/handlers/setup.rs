use super::{CommandManager, ComponentManager, EventManager};
use crate::commands::{setup_commands, HelpCommand};
use crate::components::setup_components;
use crate::events::setup_events;
use std::sync::Arc;
use twilight_gateway::Intents;

pub struct HandlersSetup {
    pub command_manager: Arc<CommandManager>,
    pub event_manager: Arc<EventManager>,
    pub component_manager: Arc<ComponentManager>,
    pub help_command: Arc<HelpCommand>,
}

impl HandlersSetup {
    pub fn new() -> Self {
        let mut cmd_mgr = CommandManager::new();
        setup_commands(&mut cmd_mgr);
        cmd_mgr.log_summary();
        let cmd_mgr = Arc::new(cmd_mgr);

        let help_cmd = Arc::new(HelpCommand::new().with_manager(Arc::clone(&cmd_mgr)));

        let mut evt_mgr = EventManager::new();
        setup_events(&mut evt_mgr);
        evt_mgr.log_summary();
        let evt_mgr = Arc::new(evt_mgr);

        let mut comp_manager = ComponentManager::new();
        setup_components(&mut comp_manager, &cmd_mgr);
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

pub fn get_default_intents() -> Intents {
    Intents::GUILDS
        | Intents::GUILD_MESSAGES
        | Intents::GUILD_MESSAGE_REACTIONS
        | Intents::MESSAGE_CONTENT
        | Intents::GUILD_MEMBERS
        | Intents::DIRECT_MESSAGES
}
