use super::utility::{HelpCommand, PingCommand, UserInfoCommand};
use crate::handlers::CommandManager;
use std::sync::Arc;

/// Setup dan register semua commands ke CommandManager
pub fn setup_commands(cmd_mgr: &mut CommandManager) {
    // Register utility commands
    cmd_mgr.register("utility", Arc::new(PingCommand));
    cmd_mgr.register("utility", Arc::new(UserInfoCommand));
    cmd_mgr.register("utility", Arc::new(HelpCommand::new()));

    // TODO: Register fun commands
    // TODO: Register moderation commands
}
