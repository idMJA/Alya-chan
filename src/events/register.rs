use super::{GuildCreateHandler, MessageCreateHandler, ReadyHandler};
use crate::handlers::EventManager;
use std::sync::Arc;

pub fn setup_events(evt_mgr: &mut EventManager) {
    evt_mgr.register(Arc::new(ReadyHandler));
    evt_mgr.register(Arc::new(MessageCreateHandler));
    evt_mgr.register(Arc::new(GuildCreateHandler));
}
