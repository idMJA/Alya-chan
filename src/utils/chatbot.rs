use crate::database::service::AlyaDatabase;
use crate::types::error::BotError;
use crate::types::{BotResult, EventContext};
use base64::{engine::general_purpose, Engine as _};
use chrono::{DateTime, Utc};
use serde_json::json;
use std::collections::HashMap;
use std::fs;
use std::sync::Arc;
use tokio::sync::RwLock;
use twilight_model::gateway::payload::incoming::MessageCreate;

const MAX_HISTORY_MESSAGES: usize = 10;
const HISTORY_TTL_MINUTES: i64 = 10;
const MAX_AUDIO_BYTES: usize = 5 * 1024 * 1024; // 5 MiB guardrail for audio downloads

// In-memory chat history storage
type ChatHistory = Arc<RwLock<HashMap<String, Vec<(String, String, DateTime<Utc>)>>>>;

static CHAT_HISTORY: tokio::sync::OnceCell<ChatHistory> = tokio::sync::OnceCell::const_new();

fn load_alya_system_message() -> String {
    fs::read_to_string("src/models/alya-id.txt").unwrap_or_else(|e| {
        tracing::warn!("Failed to load alya-id.txt: {}, using fallback", e);

        "You are Alya, a helpful and friendly Discord bot assistant. You are cheerful, knowledgeable, and always ready to help users with their questions. Keep responses concise and friendly.".to_string()
    })
}

#[allow(clippy::too_many_lines)]
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

    let Some(guild_id) = msg.guild_id else {
        return Ok(());
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

    let is_replying_to_bot = msg
        .referenced_message
        .as_ref()
        .is_some_and(|referenced_msg| referenced_msg.author.id == ctx.bot.bot_user.id);

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

    // Build user message content (include replied message if exists)
    let user_message_content = msg.referenced_message.as_ref().map_or_else(
        || msg.content.clone(),
        |referenced_msg| {
            let replied_content = &referenced_msg.content;
            let replied_author = &referenced_msg.author.name;
            format!(
                "[Replying to {}: \"{}\"]\n\n{}",
                replied_author, replied_content, msg.content
            )
        },
    );

    let mut content_parts = vec![json!({
        "type": "text",
        "text": user_message_content.clone(),
    })];

    let mut attachment_notes = Vec::new();

    let client = reqwest::Client::new();

    for attachment in &msg.attachments {
        let content_type = attachment.content_type.as_deref();
        let filename = attachment.filename.as_str();
        let url = &attachment.url;

        if is_image(content_type, filename) {
            content_parts.push(json!({
                "type": "image_url",
                "image_url": { "url": url }
            }));
            attachment_notes.push(format!("image: {}", filename));
            continue;
        }

        if is_video(content_type, filename) {
            content_parts.push(json!({
                "type": "video_url",
                "video_url": { "url": url }
            }));
            attachment_notes.push(format!("video: {}", filename));
            continue;
        }

        if is_audio(content_type, filename) {
            match fetch_audio_base64(&client, url, content_type, filename).await {
                Ok((data, format)) => {
                    content_parts.push(json!({
                        "type": "input_audio",
                        "input_audio": {
                            "data": data,
                            "format": format
                        }
                    }));
                    attachment_notes.push(format!("audio: {}", filename));
                }
                Err(err) => {
                    tracing::warn!("Audio attachment skipped ({}): {}", filename, err);
                }
            }
        }
    }

    let user_message_payload_content = if content_parts.len() == 1 {
        json!(user_message_content)
    } else {
        json!(content_parts)
    };

    let mut user_message_for_history = user_message_content.clone();
    if !attachment_notes.is_empty() {
        user_message_for_history.push_str("\n[attachments: ");
        user_message_for_history.push_str(&attachment_notes.join(", "));
        user_message_for_history.push(']');
    }

    let now = Utc::now();

    // Append current user message to payload
    messages.push(json!({
        "role": "user",
        "content": user_message_payload_content
    }));

    // Store user message in memory
    {
        let mut history_map = history_store.write().await;
        history_map
            .entry(user_key.clone())
            .or_insert_with(Vec::new)
            .push(("user".to_string(), user_message_for_history.clone(), now));
    }

    let _ = ctx.bot.http.create_typing_trigger(msg.channel_id).await;

    let client = reqwest::Client::new();

    let request_body = json!({
        "model": "openrouter/auto",
        "messages": messages,
        "temperature": 0.7,
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

fn is_image(content_type: Option<&str>, filename: &str) -> bool {
    let ext = file_ext(filename);
    matches!(
        ext.as_deref(),
        Some("png" | "jpg" | "jpeg" | "gif" | "webp" | "bmp" | "tiff")
    ) || content_type.is_some_and(|ct| ct.starts_with("image/"))
}

fn is_video(content_type: Option<&str>, filename: &str) -> bool {
    let ext = file_ext(filename);
    matches!(ext.as_deref(), Some("mp4" | "mov" | "webm" | "mkv" | "avi"))
        || content_type.is_some_and(|ct| ct.starts_with("video/"))
}

fn is_audio(content_type: Option<&str>, filename: &str) -> bool {
    let ext = file_ext(filename);
    matches!(ext.as_deref(), Some("mp3" | "wav" | "flac" | "ogg" | "m4a"))
        || content_type.is_some_and(|ct| ct.starts_with("audio/"))
}

fn file_ext(filename: &str) -> Option<String> {
    filename
        .rsplit('.')
        .next()
        .map(|ext| ext.to_ascii_lowercase())
}

fn audio_format_from(content_type: Option<&str>, filename: &str) -> String {
    if let Some(ct) = content_type {
        if let Some(short) = ct.strip_prefix("audio/") {
            return short.to_string();
        }
    }

    file_ext(filename).unwrap_or_else(|| "wav".to_string())
}

async fn fetch_audio_base64(
    client: &reqwest::Client,
    url: &str,
    content_type: Option<&str>,
    filename: &str,
) -> Result<(String, String), String> {
    let resp = client.get(url).send().await.map_err(|e| e.to_string())?;

    if let Some(len) = resp.content_length() {
        if len as usize > MAX_AUDIO_BYTES {
            return Err(format!("audio too large ({} bytes)", len));
        }
    }

    let bytes = resp.bytes().await.map_err(|e| e.to_string())?;
    if bytes.len() > MAX_AUDIO_BYTES {
        return Err(format!("audio too large ({} bytes)", bytes.len()));
    }

    let format = audio_format_from(content_type, filename);
    Ok((general_purpose::STANDARD.encode(bytes), format))
}
