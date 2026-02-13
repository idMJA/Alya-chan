use crate::types::context::BotContext;
use crate::utils::vote::create_topgg_vote_handler;
use anyhow::Result;
use axum::{http::StatusCode, response::IntoResponse, Router};
use std::sync::Arc;
use tokio::net::TcpListener;

/// API server for handling webhooks (top.gg votes, etc)
pub struct WebhookServer {
    listener: TcpListener,
}

/// Health check handler
async fn health() -> impl IntoResponse {
    (StatusCode::OK, "OK")
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
        _bot_context: Arc<BotContext>,
        _webhook_auth: String,
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

        // Create topgg webhook router (mounted at /)
        let topgg_router = topgg::axum::webhook(webhook_auth, Arc::clone(&vote_handler));

        // Create main router with health check at / and vote webhook at /vote
        let router = Router::new()
            .route("/", axum::routing::get(health))
            .nest("/vote", topgg_router);

        axum::serve(self.listener, router.into_make_service()).await?;

        Ok(())
    }
}
