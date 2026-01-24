mod chatbot;
mod globalchat;

pub use chatbot::ChatbotCommand;
pub use globalchat::GlobalChatCommand;

use super::informations::AboutCommand;
use super::utility::{HelpCommand, PingCommand, UserInfoCommand};
use crate::handlers::CommandManager;
use std::sync::Arc;

pub fn setup_commands(cmd_mgr: &mut CommandManager) {
    cmd_mgr.register("informations", Arc::new(AboutCommand));

    cmd_mgr.register("utility", Arc::new(PingCommand));
    cmd_mgr.register("utility", Arc::new(UserInfoCommand));
    cmd_mgr.register("utility", Arc::new(HelpCommand::new()));

    cmd_mgr.register("setup", Arc::new(ChatbotCommand));
    cmd_mgr.register("setup", Arc::new(GlobalChatCommand));

    // TODO: Register fun commands
    // TODO: Register moderation commands
}
