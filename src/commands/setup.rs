use super::utility::{HelpCommand, PingCommand, UserInfoCommand};
use crate::handlers::CommandManager;
use std::sync::Arc;

pub fn setup_commands(cmd_mgr: &mut CommandManager) {
    cmd_mgr.register("utility", Arc::new(PingCommand));
    cmd_mgr.register("utility", Arc::new(UserInfoCommand));
    cmd_mgr.register("utility", Arc::new(HelpCommand::new()));

    // TODO: Register fun commands
    // TODO: Register moderation commands
}
