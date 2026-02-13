use crate::types::context::BotContext;
use anyhow::{anyhow, Result};
use serde_json::json;
use std::num::NonZeroU64;
use std::sync::Arc;
use topgg::{Vote, VoteHandler};

pub struct AlyaVoteHandler {
    pub context: Arc<BotContext>,
}

async fn send_vote_webhook(context: &Arc<BotContext>, voter_id: u64) {
    let webhook_url = match &context.config.webhook {
        Some(wh) => wh.vote_log.as_ref(),
        None => return,
    };

    let Some(webhook_url) = webhook_url else {
        return;
    };

    let voter = match context
        .http
        .user(
            NonZeroU64::new(voter_id)
                .expect("voter_id should be non-zero")
                .into(),
        )
        .await
    {
        Ok(response) => match response.model().await {
            Ok(user) => user,
            Err(e) => {
                tracing::warn!("[Top.gg] Failed to get voter user model: {}", e);
                return;
            }
        },
        Err(e) => {
            tracing::warn!("[Top.gg] Failed to fetch voter user: {}", e);
            return;
        }
    };

    let voter_avatar_url = if let Some(avatar) = &voter.avatar {
        format!(
            "https://cdn.discordapp.com/avatars/{}/{}.webp",
            voter.id, avatar
        )
    } else {
        format!(
            "https://cdn.discordapp.com/embed/avatars/{}.png",
            voter.id.get() % 5
        )
    };

    let bot_avatar_url = context.bot_user.avatar.as_ref().map(|avatar| {
        format!(
            "https://cdn.discordapp.com/avatars/{}/{}.webp",
            context.bot_user.id, avatar
        )
    });

    let author_name = voter.global_name.as_ref().unwrap_or(&voter.name);

    let embed = json!({
        "color": context.config.color.primary,
        "author": {
            "name": author_name,
            "icon_url": voter_avatar_url,
        },
        "thumbnail": {
            "url": voter_avatar_url
        },
        "description": format!(
            "{} **{}** `({})` just rocked the vote for Alya on [Top.gg]({})!\n\nYou're awesome for choosing us! May your day be filled with fantastic tunes and good vibes. Let's keep the music playing!",
            context.config.emoji.party,
            voter.name,
            voter.id,
            context.config.info.vote_url
        ),
        "footer": {
            "text": "Thanks for choosing Alya!"
        },
        "timestamp": chrono::Utc::now().to_rfc3339()
    });

    let mut payload = json!({
        "username": &context.bot_user.name,
        "content": format!("<@{}>", voter_id),
        "embeds": [embed]
    });

    if let Some(avatar_url) = bot_avatar_url {
        payload["avatar_url"] = json!(avatar_url);
    }

    if let Err(e) = reqwest::Client::new()
        .post(webhook_url)
        .json(&payload)
        .send()
        .await
    {
        tracing::warn!("[Top.gg] Failed to send vote webhook: {}", e);
    }
}

#[async_trait::async_trait]
impl VoteHandler for AlyaVoteHandler {
    async fn voted(&self, vote: Vote) {
        tracing::info!(
            "[Top.gg] Received vote: voter_id={} receiver_id={} is_server={} is_test={} is_weekend={}",
            vote.voter_id,
            vote.receiver_id,
            vote.is_server,
            vote.is_test,
            vote.is_weekend
        );

        if vote.is_test {
            tracing::info!("[Top.gg] Skipped test webhook from user {}", vote.voter_id);
            return;
        }

        let user_id = vote.voter_id.to_string();
        match self.context.database.add_user_vote(&user_id).await {
            Ok(()) => {
                tracing::info!("[Top.gg] Successfully recorded vote for user {}", user_id);
            }
            Err(e) => {
                tracing::error!("[Top.gg] Failed to record vote for user {}: {}", user_id, e);
                return;
            }
        }

        send_vote_webhook(&self.context, vote.voter_id).await;
    }
}

pub const fn create_topgg_vote_handler(context: Arc<BotContext>) -> AlyaVoteHandler {
    AlyaVoteHandler { context }
}

#[allow(dead_code)]
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

    if provided != cfg.webhook_auth {
        return Err(anyhow!("invalid top.gg webhook auth token"));
    }

    let vote: Vote =
        serde_json::from_str(body).map_err(|e| anyhow!("failed to parse top.gg payload: {e}"))?;

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

    ctx.database
        .add_user_vote(&vote.voter_id.to_string())
        .await?;
    tracing::info!("[Top.gg] Recorded vote for user {}", vote.voter_id);

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
