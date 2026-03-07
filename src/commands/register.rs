use super::fun::{FakeTweetCommand, HackCommand, IqCommand, ShipCommand, WaifuCommand};
use super::informations::{AboutCommand, HelpCommand, PingCommand};
use super::moderation::{BanCommand, KickCommand, TimeoutCommand};
use super::setup::{
    ChatbotCommand, ChatbotLanguageCommand, ChatbotLocaleCommand, GlobalChatCommand,
};
use super::utility::{DigCommand, MultiDigCommand, UserInfoCommand, WhoisCommand};
use crate::handlers::CommandManager;
use crate::types::command::SlashCommand;
use std::sync::Arc;

pub fn setup_commands(cmd_mgr: &mut CommandManager) {
    cmd_mgr.register(
        "informations",
        &(Arc::new(AboutCommand) as Arc<dyn SlashCommand>),
    );
    cmd_mgr.register(
        "informations",
        &(Arc::new(HelpCommand::new()) as Arc<dyn SlashCommand>),
    );

    cmd_mgr.register("utility", &(Arc::new(PingCommand) as Arc<dyn SlashCommand>));
    cmd_mgr.register(
        "utility",
        &(Arc::new(UserInfoCommand) as Arc<dyn SlashCommand>),
    );
    cmd_mgr.register("utility", &(Arc::new(DigCommand) as Arc<dyn SlashCommand>));
    cmd_mgr.register(
        "utility",
        &(Arc::new(MultiDigCommand) as Arc<dyn SlashCommand>),
    );
    cmd_mgr.register(
        "utility",
        &(Arc::new(WhoisCommand) as Arc<dyn SlashCommand>),
    );

    cmd_mgr.register(
        "setup",
        &(Arc::new(ChatbotCommand) as Arc<dyn SlashCommand>),
    );
    cmd_mgr.register(
        "setup",
        &(Arc::new(ChatbotLanguageCommand) as Arc<dyn SlashCommand>),
    );
    cmd_mgr.register(
        "setup",
        &(Arc::new(ChatbotLocaleCommand) as Arc<dyn SlashCommand>),
    );
    cmd_mgr.register(
        "setup",
        &(Arc::new(GlobalChatCommand) as Arc<dyn SlashCommand>),
    );

    cmd_mgr.register(
        "fun",
        &(Arc::new(FakeTweetCommand) as Arc<dyn SlashCommand>),
    );
    cmd_mgr.register("fun", &(Arc::new(HackCommand) as Arc<dyn SlashCommand>));
    cmd_mgr.register("fun", &(Arc::new(IqCommand) as Arc<dyn SlashCommand>));
    cmd_mgr.register("fun", &(Arc::new(ShipCommand) as Arc<dyn SlashCommand>));
    cmd_mgr.register("fun", &(Arc::new(WaifuCommand) as Arc<dyn SlashCommand>));

    cmd_mgr.register(
        "moderation",
        &(Arc::new(BanCommand) as Arc<dyn SlashCommand>),
    );
    cmd_mgr.register(
        "moderation",
        &(Arc::new(KickCommand) as Arc<dyn SlashCommand>),
    );
    cmd_mgr.register(
        "moderation",
        &(Arc::new(TimeoutCommand) as Arc<dyn SlashCommand>),
    );
}
