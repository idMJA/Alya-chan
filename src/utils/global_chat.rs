use crate::types::{BotResult, EventContext};
use serde_json::json;
use twilight_model::gateway::payload::incoming::MessageCreate;

fn build_safe_message(
    msg: &MessageCreate,
    cache: &twilight_cache_inmemory::InMemoryCache,
) -> serde_json::Value {
    let author = &msg.author;

    let guild_name = msg
        .guild_id
        .and_then(|gid| cache.guild(gid))
        .map(|g| g.name().to_string());

    let referenced = msg
        .reference
        .as_ref()
        .and_then(|_| msg.referenced_message.as_ref());

    let attachments = msg
        .attachments
        .iter()
        .map(|a| {
            json!({
                "url": a.url,
                "contentType": a.content_type,
            })
        })
        .collect::<Vec<_>>();

    let stickers: Vec<serde_json::Value> = msg
        .sticker_items
        .iter()
        .map(|s| {
            json!({
                "id": s.id,
                "name": s.name,
                "formatType": s.format_type,
            })
        })
        .collect();

    let author_avatar_url = format!(
        "https://cdn.discordapp.com/avatars/{}/{}",
        author.id,
        author
            .avatar
            .as_ref()
            .map_or_else(|| "default".to_string(), std::string::ToString::to_string)
    );

    json!({
        "id": msg.id,
        "content": msg.content,
        "author": {
            "id": author.id,
            "username": author.name,
            "globalName": author.global_name.as_deref().unwrap_or(""),
            "avatarURL": author_avatar_url,
        },
        "channelId": msg.channel_id,
        "guildId": msg.guild_id,
        "guildName": guild_name,
        "referencedMessage": referenced.map(|rm| json!({
            "id": rm.id,
            "content": rm.content,
            "author": {
                "id": rm.author.id,
                "username": rm.author.name,
                "globalName": rm.author.global_name.as_deref().unwrap_or(""),
            }
        })),
        "attachments": attachments,
        "stickerItems": stickers,
    })
}

#[allow(clippy::too_many_lines)]
pub async fn handle_global_chat(ctx: &EventContext, msg: &MessageCreate) -> BotResult<()> {
    let gc_config = match &ctx.bot.config.global_chat {
        Some(gc) if gc.enabled => gc,
        _ => return Ok(()),
    };

    if msg.author.bot {
        return Ok(());
    }

    let payload = build_safe_message(msg, &ctx.bot.cache);
    let body = json!({
        "message": payload,
        "guildName": payload.get("guildName").cloned(),
    });

    let client = reqwest::Client::new();
    let mut req = client
        .post(format!("{}/chat", gc_config.api_url))
        .header("Content-Type", "application/json")
        .json(&body);

    if let Some(key) = &gc_config.api_key {
        req = req.header("Authorization", format!("Bearer {key}"));
    }

    let resp = req
        .send()
        .await
        .map_err(|e| crate::types::error::BotError::Other(e.to_string()))?;
    let status_code = resp.status();
    let result: serde_json::Value = resp
        .json()
        .await
        .map_err(|e| crate::types::error::BotError::Other(e.to_string()))?;

    let status = result
        .get("status")
        .and_then(|s| s.as_str())
        .unwrap_or("unknown");
    let data = result.get("data").cloned().unwrap_or_else(|| json!({}));

    match status {
        "ok" => {
            let total = data
                .get("deliveryStats")
                .and_then(|ds| ds.get("total"))
                .and_then(serde_json::Value::as_u64)
                .unwrap_or(0);
            let rate = data
                .get("deliveryStats")
                .and_then(|ds| ds.get("successRate"))
                .and_then(serde_json::Value::as_f64)
                .unwrap_or(0.0);
            tracing::info!("Message broadcasted successfully to {} servers", total);
            tracing::info!("Success rate: {}%", rate);
        }
        "ignored" => {
            let reason = data
                .get("reason")
                .and_then(|r| r.as_str())
                .unwrap_or("Not from global chat channel");
            tracing::info!("Message ignored: {}", reason);
        }
        "skipped" => {}
        "partial" => {
            let successful = data
                .get("deliveryStats")
                .and_then(|ds| ds.get("successful"))
                .and_then(serde_json::Value::as_u64)
                .unwrap_or(0);
            let total = data
                .get("deliveryStats")
                .and_then(|ds| ds.get("total"))
                .and_then(serde_json::Value::as_u64)
                .unwrap_or(0);
            tracing::info!("Partially delivered: {}/{}", successful, total);

            if let Some(failed) = data.get("failedGuilds").and_then(|v| v.as_array()) {
                let names = failed
                    .iter()
                    .filter_map(|g| g.get("guildName").and_then(|n| n.as_str()))
                    .collect::<Vec<_>>()
                    .join(", ");
                if !names.is_empty() {
                    tracing::warn!("Failed guilds: {}", names);
                }

                let failed_guilds = failed.clone();
                let http = ctx.bot.http.clone();
                let api_url = gc_config.api_url.clone();
                let api_key = gc_config.api_key.clone();
                tokio::spawn(async move {
                    if let Err(e) =
                        handle_failed_guilds(&failed_guilds, &http, &api_url, api_key).await
                    {
                        tracing::error!("Failed to process failed guilds: {:?}", e);
                    }
                });
            }
        }
        "failed" => {
            tracing::error!(
                "All deliveries failed for message {} (HTTP {})",
                msg.id,
                status_code
            );
            if let Some(failed) = data.get("failedGuilds").and_then(|v| v.as_array()) {
                let errs = failed
                    .iter()
                    .filter_map(|g| g.get("error").and_then(|e| e.as_str()))
                    .collect::<Vec<_>>()
                    .join("; ");
                if !errs.is_empty() {
                    tracing::error!("Failed guilds: {}", errs);
                }

                let failed_guilds = failed.clone();
                let http = ctx.bot.http.clone();
                let api_url = gc_config.api_url.clone();
                let api_key = gc_config.api_key.clone();
                tokio::spawn(async move {
                    if let Err(e) =
                        handle_failed_guilds(&failed_guilds, &http, &api_url, api_key).await
                    {
                        tracing::error!("Failed to process failed guilds: {:?}", e);
                    }
                });
            }
        }
        _ => {
            tracing::warn!("Unknown response status: {}", status);
            tracing::info!("Full response: {}", result);
        }
    }

    if matches!(status, "ok" | "partial") {
        if let Some(gid) = msg.guild_id {
            let gname = payload
                .get("guildName")
                .and_then(|n| n.as_str())
                .unwrap_or("");
            tracing::info!("📤 From guild: {} ({})", gname, gid);
        }
    }

    Ok(())
}

#[allow(clippy::too_many_lines)]
async fn handle_failed_guilds(
    failed_guilds: &[serde_json::Value],
    http: &std::sync::Arc<twilight_http::Client>,
    api_url: &str,
    api_key: Option<String>,
) -> BotResult<()> {
    for failed in failed_guilds {
        let guild_id_str = failed
            .get("guildId")
            .and_then(|id| id.as_str())
            .unwrap_or_default();
        let guild_name = failed
            .get("guildName")
            .and_then(|n| n.as_str())
            .unwrap_or("Unknown");

        if guild_id_str.is_empty() {
            continue;
        }

        // tracing::info!(
        //     "🔧 Attempting to fix webhook for guild {} ({})",
        //     guild_name,
        //     guild_id_str
        // );

        let mut headers = reqwest::header::HeaderMap::new();
        headers.insert("Content-Type", "application/json".parse().unwrap());
        if let Some(key) = &api_key {
            headers.insert("Authorization", format!("Bearer {key}").parse().unwrap());
        }

        let client = reqwest::Client::new();
        let guild_list_resp = client
            .get(format!("{api_url}/list"))
            .headers(headers.clone())
            .send()
            .await;

        let guild_list_resp = match guild_list_resp {
            Ok(resp) => resp,
            Err(e) => {
                tracing::error!(
                    "❌ Failed to fetch guild list from API for {}: {}",
                    guild_name,
                    e
                );
                continue;
            }
        };

        let list_data: serde_json::Value = match guild_list_resp.json().await {
            Ok(data) => data,
            Err(e) => {
                tracing::error!(
                    "❌ Failed to parse guild list response for {}: {}",
                    guild_name,
                    e
                );
                continue;
            }
        };

        let guild_info = list_data
            .get("data")
            .and_then(|d| d.get("guilds"))
            .and_then(|g| g.as_array())
            .and_then(|guilds| {
                guilds
                    .iter()
                    .find(|g| g.get("id").and_then(|id| id.as_str()) == Some(guild_id_str))
            });

        let global_channel_id =
            if let Some(id) = guild_info.and_then(|gi| gi.get("globalChannelId")) {
                if let Some(s) = id.as_str() {
                    s
                } else {
                    tracing::warn!(
                        "❌ Could not find valid global channel ID for guild {}",
                        guild_id_str
                    );
                    continue;
                }
            } else {
                tracing::warn!(
                    "❌ Could not find guild info for {} ({})",
                    guild_name,
                    guild_id_str
                );
                continue;
            };

        let channel_id = if let Ok(id) = global_channel_id.parse::<u64>() {
            twilight_model::id::Id::new(id)
        } else {
            tracing::error!(
                "❌ Invalid channel ID format for guild {}: {}",
                guild_name,
                global_channel_id
            );
            continue;
        };

        let webhook_result = http.create_webhook(channel_id, "Alya Global Chat").await;

        let webhook_resp = match webhook_result {
            Ok(resp) => resp,
            Err(_e) => {
                // tracing::error!(
                //     "❌ Failed to create webhook for guild {}: {}",
                //     guild_name,
                //     e
                // );
                continue;
            }
        };

        let webhook = webhook_resp.model().await;
        let webhook = match webhook {
            Ok(wh) => wh,
            Err(e) => {
                tracing::error!("❌ Failed to parse webhook for guild {}: {}", guild_name, e);
                continue;
            }
        };

        let update_body = json!({
            "guildId": guild_id_str,
            "globalChannelId": global_channel_id,
            "webhookId": webhook.id.to_string(),
            "webhookToken": webhook.token,
        });

        let update_resp = client
            .post(format!("{api_url}/add"))
            .headers(headers.clone())
            .json(&update_body)
            .send()
            .await;

        let update_resp = match update_resp {
            Ok(resp) => resp,
            Err(e) => {
                tracing::error!(
                    "❌ Failed to send webhook update for guild {}: {}",
                    guild_name,
                    e
                );
                continue;
            }
        };

        let update_result: serde_json::Value = match update_resp.json().await {
            Ok(data) => data,
            Err(e) => {
                tracing::error!(
                    "❌ Failed to parse webhook update response for {}: {}",
                    guild_name,
                    e
                );
                continue;
            }
        };

        if update_result.get("status").and_then(|s| s.as_str()) == Some("ok") {
            tracing::info!("✅ Successfully fixed webhook for guild {}", guild_name);
        } else {
            tracing::error!(
                "❌ Failed to update guild {} in API: {:?}",
                guild_name,
                update_result.get("error")
            );
        }
    }

    Ok(())
}
