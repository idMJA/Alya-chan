use crate::database::service::AlyaDatabase;
use crate::types::error::BotError;
use crate::types::{BotResult, EventContext};
use chrono::{DateTime, Utc};
use serde_json::json;
use std::collections::HashMap;
use std::fs;
use std::sync::Arc;
use tokio::sync::RwLock;
use twilight_model::gateway::payload::incoming::MessageCreate;

const MAX_HISTORY_MESSAGES: usize = 10;
const HISTORY_TTL_MINUTES: i64 = 10;

// In-memory chat history storage
type ChatHistory = Arc<RwLock<HashMap<String, Vec<(String, String, DateTime<Utc>)>>>>;

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

    tracing::debug!(
        "Chatbot setup for guild {}: {:?}",
        guild_id,
        setup.is_some()
    );

    let content_lower = msg.content.to_lowercase();
    let is_alya_mentioned = content_lower
        .split(|c: char| !c.is_alphanumeric())
        .any(|word| word == "alya");

    tracing::debug!(
        "Message from {} in channel {}: alya_mentioned={}, has_setup={}",
        msg.author.id,
        msg.channel_id,
        is_alya_mentioned,
        setup.is_some()
    );

    let is_bot_mentioned = msg.mentions.iter().any(|u| u.id == ctx.bot.bot_user.id);

    let is_replying_to_bot = if let Some(referenced_msg) = &msg.referenced_message {
        referenced_msg.author.id == ctx.bot.bot_user.id
    } else {
        false
    };

    let should_respond = if is_alya_mentioned || is_bot_mentioned || is_replying_to_bot {
        tracing::debug!("Responding: alya mentioned, bot mentioned, or replied");
        true
    } else if let Some(existing) = &setup {
        let channel_match = existing.channel_id == msg.channel_id.to_string();
        tracing::debug!(
            "Channel check: stored='{}', msg='{}', match={}",
            existing.channel_id,
            msg.channel_id,
            channel_match
        );
        channel_match
    } else {
        tracing::debug!("No setup found, no mentions/replies - skipping");
        false
    };

    if !should_respond {
        return Ok(());
    }

    let system_message = load_alya_system_message();

    // Use in-memory chat history (no DB persistence)
    static CHAT_HISTORY: tokio::sync::OnceCell<ChatHistory> = tokio::sync::OnceCell::const_new();
    let history_store = CHAT_HISTORY
        .get_or_init(|| async { Arc::new(RwLock::new(HashMap::new())) })
        .await;

    // Fetch recent history from memory
    let mut messages = vec![json!({
        "role": "system",
        "content": system_message
    })];

    let user_key = msg.author.id.to_string();
    let cutoff_time = Utc::now() - chrono::Duration::minutes(HISTORY_TTL_MINUTES);

    {
        let history_map = history_store.read().await;
        if let Some(user_history) = history_map.get(&user_key) {
            for (role, content, timestamp) in
                user_history.iter().rev().take(MAX_HISTORY_MESSAGES).rev()
            {
                if *timestamp > cutoff_time {
                    messages.push(json!({
                        "role": role,
                        "content": content
                    }));
                }
            }
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

    // Store user message in memory
    {
        let mut history_map = history_store.write().await;
        history_map
            .entry(user_key.clone())
            .or_insert_with(Vec::new)
            .push(("user".to_string(), user_message_content.clone(), now));
    }

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

                    // Store assistant response in memory
                    {
                        let mut history_map = history_store.write().await;
                        history_map
                            .entry(user_key.clone())
                            .or_insert_with(Vec::new)
                            .push(("assistant".to_string(), reply.to_string(), Utc::now()));
                    }

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
