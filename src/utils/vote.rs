use crate::types::context::BotContext;
use anyhow::{anyhow, Result};
use std::sync::Arc;
use topgg::{Vote, VoteHandler};

/// Axum-compatible vote handler for top.gg webhooks
pub struct AlyaVoteHandler {
    pub context: Arc<BotContext>,
}

#[async_trait::async_trait]
impl VoteHandler for AlyaVoteHandler {
    /// Called when a vote is received and authenticated.
    /// Automatically handles authentication and 200 OK response.
    async fn voted(&self, vote: Vote) {
        tracing::info!(
            "[Top.gg] Received vote: voter_id={} receiver_id={} is_server={} is_test={} is_weekend={}",
            vote.voter_id,
            vote.receiver_id,
            vote.is_server,
            vote.is_test,
            vote.is_weekend
        );

        // Skip recording test votes
        if vote.is_test {
            tracing::info!("[Top.gg] Skipped test webhook from user {}", vote.voter_id);
            return;
        }

        // Record vote in database
        let user_id = vote.voter_id.to_string();
        match self.context.database.add_user_vote(&user_id).await {
            Ok(_) => {
                tracing::info!("[Top.gg] Successfully recorded vote for user {}", user_id);
            }
            Err(e) => {
                tracing::error!("[Top.gg] Failed to record vote for user {}: {}", user_id, e);
            }
        }

        // TODO: send a DM/notification to the user
    }
}

/// Create a vote handler for top.gg webhook events.
///
/// # Arguments
///
/// - `context` - Reference to the bot context (contains config and database)
///
/// # Returns
///
/// An `AlyaVoteHandler` that implements the `VoteHandler` trait.
/// Mount this with `topgg::axum::webhook(webhook_auth, Arc::new(handler))` at your preferred path.
///
/// # Example
///
/// ```ignore
/// let handler = create_topgg_vote_handler(context);
/// let webhook_router = topgg::axum::webhook(webhook_auth, Arc::new(handler));
/// // Mount webhook_router in your axum app
/// ```
pub fn create_topgg_vote_handler(context: Arc<BotContext>) -> AlyaVoteHandler {
    AlyaVoteHandler { context }
}

/// Handle an incoming top.gg webhook payload (deprecated: use axum integration instead).
///
/// - `body` is the raw JSON body from the webhook
/// - `auth_header` is the value of the `Authorization` header (should be `webhook_auth` from config)
///
/// Behavior:
/// - verify the webhook auth header matches `ctx.config.top_gg.webhook_auth` and that top.gg is enabled
/// - parse the payload
/// - for a real vote (not a test), record the vote in DB using `add_user_vote`
///
/// Note: Sending a DM/notification is left intentionally minimal (see TODO) to avoid
/// adding fragile dependencies here — for now we persist the vote and log it.
pub async fn handle_topgg_webhook(
    ctx: &BotContext,
    body: &str,
    auth_header: Option<&str>,
) -> Result<()> {
    let cfg = match &ctx.config.top_gg {
        Some(c) if c.enabled => c,
        _ => return Err(anyhow!("top.gg integration disabled")),
    };

    let provided = auth_header.unwrap_or("");

    // Webhook auth check: top.gg sends the webhook auth password via `Authorization` header
    if provided != cfg.webhook_auth {
        return Err(anyhow!("invalid top.gg webhook auth token"));
    }

    let vote: Vote =
        serde_json::from_str(body).map_err(|e| anyhow!("failed to parse top.gg payload: {}", e))?;

    tracing::info!(
        "[Top.gg] Received webhook: voter_id={} receiver_id={} is_server={} is_test={} is_weekend={}",
        vote.voter_id,
        vote.receiver_id,
        vote.is_server,
        vote.is_test,
        vote.is_weekend
    );

    if vote.is_test {
        tracing::info!("[Top.gg] Received test webhook from user {}", vote.voter_id);
        return Ok(());
    }

    // Record vote in DB (gives the user VOTE_PREMIUM_DURATION_HOURS premium)
    ctx.database
        .add_user_vote(&vote.voter_id.to_string())
        .await?;
    tracing::info!("[Top.gg] Recorded vote for user {}", vote.voter_id);

    // TODO: send a DM/notification to the user (requires a stable behavior
    // for creating a private channel / message builder with twilight_http). For
    // now we keep this minimal and just log + persist the vote.

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_upvote_payload() {
        let raw = r#"{"type":"upvote","user":"1234567890","bot":"987654321","isWeekend":true}"#;
        let vote: Vote = serde_json::from_str(raw).expect("should parse upvote payload");
        assert_eq!(vote.voter_id, 1_234_567_890);
        assert_eq!(vote.receiver_id, 987_654_321);
        assert!(!vote.is_test);
        assert!(vote.is_weekend);
    }

    #[test]
    fn parse_test_payload() {
        let raw = r#"{"type":"test","user":"2222","bot":"1111"}"#;
        let vote: Vote = serde_json::from_str(raw).expect("should parse test payload");
        assert_eq!(vote.voter_id, 2_222);
        assert_eq!(vote.receiver_id, 1_111);
        assert!(vote.is_test);
    }
}
