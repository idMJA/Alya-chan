use super::fun::{FakeTweetCommand, HackCommand, IqCommand, ShipCommand, WaifuCommand};
use super::informations::{AboutCommand, HelpCommand, PingCommand};
use super::moderation::{BanCommand, KickCommand, TimeoutCommand};
use super::setup::{ChatbotCommand, GlobalChatCommand};
use super::utility::{DigCommand, MultiDigCommand, UserInfoCommand, WhoisCommand};
use crate::handlers::CommandManager;
use std::sync::Arc;

pub fn setup_commands(cmd_mgr: &mut CommandManager) {
    cmd_mgr.register("informations", Arc::new(AboutCommand));
    cmd_mgr.register("informations", Arc::new(HelpCommand::new()));

    cmd_mgr.register("utility", Arc::new(PingCommand));
    cmd_mgr.register("utility", Arc::new(UserInfoCommand));
    cmd_mgr.register("utility", Arc::new(DigCommand));
    cmd_mgr.register("utility", Arc::new(MultiDigCommand));
    cmd_mgr.register("utility", Arc::new(WhoisCommand));

    cmd_mgr.register("setup", Arc::new(ChatbotCommand));
    cmd_mgr.register("setup", Arc::new(GlobalChatCommand));

    cmd_mgr.register("fun", Arc::new(FakeTweetCommand));
    cmd_mgr.register("fun", Arc::new(HackCommand));
    cmd_mgr.register("fun", Arc::new(IqCommand));
    cmd_mgr.register("fun", Arc::new(ShipCommand));
    cmd_mgr.register("fun", Arc::new(WaifuCommand));

    cmd_mgr.register("moderation", Arc::new(BanCommand));
    cmd_mgr.register("moderation", Arc::new(KickCommand));
    cmd_mgr.register("moderation", Arc::new(TimeoutCommand));
}
