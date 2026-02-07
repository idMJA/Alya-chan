use crate::types::context::BotContext;
use crate::utils::vote::create_topgg_vote_handler;
use anyhow::Result;
use axum::Router;
use std::sync::Arc;
use tokio::net::TcpListener;

/// API server for handling webhooks (top.gg votes, etc)
pub struct WebhookServer {
    listener: TcpListener,
}

impl WebhookServer {
    /// Create a new webhook server
    ///
    /// # Arguments
    ///
    /// - `addr` - Socket address to bind to (e.g., "127.0.0.1:3000")
    /// - `bot_context` - Reference to bot context
    /// - `webhook_auth` - Top.gg webhook auth password from config
    pub async fn new(
        addr: &str,
        bot_context: Arc<BotContext>,
        webhook_auth: String,
    ) -> Result<Self> {
        let listener = TcpListener::bind(addr).await?;
        let actual_addr = listener.local_addr()?;

        tracing::info!("[Webhook Server] Listening on {}", actual_addr);

        // Store router for later - we'll create it in run()
        Ok(Self { listener })
    }

    /// Start the webhook server (runs forever)
    pub async fn run(self, bot_context: Arc<BotContext>, webhook_auth: String) -> Result<()> {
        let vote_handler = Arc::new(create_topgg_vote_handler(bot_context));

        // Use topgg's native axum integration - it returns a Router
        let router = topgg::axum::webhook(webhook_auth, Arc::clone(&vote_handler));

        axum::serve(self.listener, router.into_make_service()).await?;

        Ok(())
    }
}
