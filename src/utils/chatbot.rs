use crate::database::service::AlyaDatabase;
use crate::types::error::BotError;
use crate::types::{BotResult, EventContext};
use chrono::Utc;
use serde_json::json;
use std::fs;
use twilight_model::gateway::payload::incoming::MessageCreate;

const MAX_HISTORY_MESSAGES: usize = 10;
const HISTORY_TTL_MINUTES: i64 = 10;

fn load_alya_system_message() -> String {
    match fs::read_to_string("src/models/alya-multi.txt") {
        Ok(content) => content,
        Err(e) => {
            tracing::warn!("Failed to load alya-multi.txt: {}, using fallback", e);

            "You are Alya, a helpful and friendly Discord bot assistant. You are cheerful, knowledgeable, and always ready to help users with their questions. Keep responses concise and friendly.".to_string()
        }
    }
}

pub async fn handle_chatbot(ctx: &EventContext, msg: &MessageCreate) -> BotResult<()> {
    let chatbot_config = match &ctx.bot.config.chatbot {
        Some(cb) if cb.enabled => cb,
        _ => return Ok(()),
    };

    if msg.author.bot {
        return Ok(());
    }

    if msg.content.len() < 3 {
        return Ok(());
    }

    let guild_id = match msg.guild_id {
        Some(id) => id,
        None => return Ok(()),
    };

    let db = AlyaDatabase::get().map_err(|e| BotError::Other(e.to_string()))?;
    let setup = db
        .get_chatbot_setup(&guild_id.to_string())
        .await
        .map_err(|e| BotError::Other(e.to_string()))?;

    let is_alya_mentioned = msg.content.to_lowercase().contains("alya");

    let is_bot_mentioned = if let Some(bot_user) = ctx.bot.cache.current_user() {
        msg.mentions.iter().any(|u| u.id == bot_user.id)
    } else {
        !msg.mentions.is_empty() && msg.mentions.iter().any(|u| u.bot)
    };

    let is_replying_to_bot = if let Some(referenced_msg) = &msg.referenced_message {
        if let Some(bot_user) = ctx.bot.cache.current_user() {
            referenced_msg.author.id == bot_user.id
        } else {
            referenced_msg.author.bot
        }
    } else {
        false
    };

    let should_respond = if is_alya_mentioned || is_bot_mentioned || is_replying_to_bot {
        true
    } else if let Some(existing) = &setup {
        existing.channel_id == msg.channel_id.to_string()
    } else {
        false
    };

    if !should_respond {
        return Ok(());
    }

    let system_message = load_alya_system_message();

    // Get singleton store (initialized in main.rs)
    let store = AlyaDatabase::get_store()
        .await
        .map_err(|e| BotError::Other(e.to_string()))?;

    // Fetch recent history (local first)
    let mut messages = vec![json!({
        "role": "system",
        "content": system_message
    })];

    if let Ok(history) = store
        .fetch_recent(
            &msg.author.id.to_string(),
            HISTORY_TTL_MINUTES,
            MAX_HISTORY_MESSAGES,
        )
        .await
    {
        for (role, content) in history {
            messages.push(json!({
                "role": role,
                "content": content
            }));
        }
    }

    let now = Utc::now();

    // Build user message content (include replied message if exists)
    let mut user_message_content = msg.content.clone();

    if let Some(referenced_msg) = &msg.referenced_message {
        let replied_content = &referenced_msg.content;
        let replied_author = &referenced_msg.author.name;
        user_message_content = format!(
            "[Replying to {}: \"{}\"]\n\n{}",
            replied_author, replied_content, msg.content
        );
    }

    // Append current user message to payload
    messages.push(json!({
        "role": "user",
        "content": user_message_content
    }));

    // Persist user message (local then remote)
    let _ = store
        .append_message(
            &msg.author.id.to_string(),
            &guild_id.to_string(),
            "user",
            &user_message_content,
            now,
        )
        .await;

    let _ = ctx.bot.http.create_typing_trigger(msg.channel_id).await;

    let client = reqwest::Client::new();

    let request_body = json!({
        "model": "meta-llama/llama-3.1-8b-instruct",
        "messages": messages,
        "metadata": {
            "guild_id": guild_id.to_string(),
            "channel_id": msg.channel_id.to_string(),
        }
    });

    let mut headers = reqwest::header::HeaderMap::new();
    headers.insert(
        reqwest::header::AUTHORIZATION,
        format!("Bearer {}", chatbot_config.api_key)
            .parse()
            .unwrap(),
    );
    headers.insert(
        reqwest::header::CONTENT_TYPE,
        "application/json".parse().unwrap(),
    );

    match client
        .post("https://openrouter.ai/api/v1/chat/completions")
        .headers(headers)
        .json(&request_body)
        .send()
        .await
    {
        Ok(response) => match response.json::<serde_json::Value>().await {
            Ok(data) => {
                if let Some(reply) = data
                    .get("choices")
                    .and_then(|c| c.as_array())
                    .and_then(|arr| arr.first())
                    .and_then(|choice| choice.get("message"))
                    .and_then(|msg| msg.get("content"))
                    .and_then(|content| content.as_str())
                {
                    let messages = split_message(reply, 2000);

                    let _ = store
                        .append_message(
                            &msg.author.id.to_string(),
                            &guild_id.to_string(),
                            "assistant",
                            reply,
                            Utc::now(),
                        )
                        .await;

                    for message_part in messages {
                        let _ = ctx.bot.http.create_typing_trigger(msg.channel_id).await;

                        ctx.bot
                            .http
                            .create_message(msg.channel_id)
                            .content(&message_part)
                            .reply(msg.id)
                            .await?;
                    }

                    tracing::info!(
                        "Chatbot responded in guild {} channel {}",
                        guild_id,
                        msg.channel_id
                    );
                } else {
                    tracing::warn!("No content in chatbot API response");
                }
            }
            Err(e) => {
                tracing::error!("Failed to parse chatbot API response: {:?}", e);

                ctx.bot
                    .http
                    .create_message(msg.channel_id)
                    .content("Sorry, I'm experiencing some issues right now.")
                    .reply(msg.id)
                    .await?;
            }
        },
        Err(e) => {
            tracing::error!("Failed to call chatbot API: {:?}", e);

            ctx.bot
                .http
                .create_message(msg.channel_id)
                .content("Sorry, I'm experiencing some issues right now.")
                .reply(msg.id)
                .await?;
        }
    }

    Ok(())
}

fn split_message(text: &str, max_length: usize) -> Vec<String> {
    let mut result = Vec::new();
    let mut start = 0;

    while start < text.len() {
        let end = (start + max_length).min(text.len());
        result.push(text[start..end].to_string());
        start = end;
    }

    result
}
