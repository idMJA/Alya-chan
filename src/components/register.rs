use super::buttons::{
    ChatbotCreateButton, DigRefreshButton, GlobalChatCreateButton, SetupDeleteButton,
};
use super::select_menus::dig_provider::DigProviderSelect;
use super::select_menus::help_menu::HelpMenuSelect;
use crate::commands::AboutButton;
use crate::components::buttons::paginator::PaginatorButton;
use crate::handlers::{CommandManager, ComponentManager};
use std::sync::Arc;

pub fn setup_components(comp_mgr: &mut ComponentManager, cmd_mgr: &Arc<CommandManager>) {
    comp_mgr.register(Arc::new(AboutButton));
    comp_mgr.register(Arc::new(SetupDeleteButton));
    comp_mgr.register(Arc::new(ChatbotCreateButton));
    comp_mgr.register(Arc::new(DigProviderSelect));
    comp_mgr.register(Arc::new(DigRefreshButton));
    comp_mgr.register(Arc::new(GlobalChatCreateButton));
    comp_mgr.register(Arc::new(HelpMenuSelect::new(Arc::clone(cmd_mgr))));
    comp_mgr.register(Arc::new(PaginatorButton::new(Arc::clone(cmd_mgr))));
}
