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
const MAX_INLINE_FILE_BYTES: usize = 8 * 1024 * 1024;
const MAX_INLINE_TOTAL_BYTES: usize = 18 * 1024 * 1024;

// In-memory chat history storage
type ChatHistory = Arc<RwLock<HashMap<String, Vec<(String, String, DateTime<Utc>)>>>>;

static CHAT_HISTORY: tokio::sync::OnceCell<ChatHistory> = tokio::sync::OnceCell::const_new();

fn normalize_chatbot_language(language: &str) -> &'static str {
    let normalized = language.trim().to_lowercase();
    match normalized.as_str() {
        "id" | "indonesia" | "indonesian" | "bahasa" | "bahasa indonesia" => "id",
        "en" | "english" | "inggris" | "bahasa inggris" => "en",
        _ => "id",
    }
}

fn locale_to_chatbot_language(locale: &str) -> &'static str {
    if locale.trim().to_lowercase().starts_with("id") {
        "id"
    } else {
        "en"
    }
}

fn fallback_system_message(language: &str) -> &'static str {
    match normalize_chatbot_language(language) {
        "en" => {
            "You are Alya, a helpful and friendly Discord bot assistant. You are cheerful, knowledgeable, and always ready to help users with their questions. Keep responses concise and friendly."
        }
        _ => {
            "kamu adalah alya, asisten bot discord yang membantu dan ramah. kamu ceria, berpengetahuan, dan selalu siap membantu pertanyaan user. balas singkat dan to the point."
        }
    }
}

fn load_alya_system_message(language: &str) -> String {
    let normalized = normalize_chatbot_language(language);
    let path = match normalized {
        "en" => "src/models/alya-en.txt",
        _ => "src/models/alya-id.txt",
    };

    fs::read_to_string(path).unwrap_or_else(|e| {
        tracing::warn!("Failed to load {}: {}, using fallback", path, e);
        fallback_system_message(normalized).to_string()
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

    let preferred_language = match db
        .get_user_chatbot_language(&msg.author.id.to_string())
        .await
    {
        Ok(Some(language)) => normalize_chatbot_language(&language).to_string(),
        Ok(None) => match db.get_chatbot_locale(&guild_id.to_string()).await {
            Ok(chatbot_locale) => normalize_chatbot_language(&chatbot_locale).to_string(),
            Err(e) => {
                tracing::warn!(
                    "Failed to fetch guild chatbot locale for {}: {}, trying guild locale",
                    guild_id,
                    e
                );
                match db.get_locale(&guild_id.to_string()).await {
                    Ok(locale) => locale_to_chatbot_language(&locale).to_string(),
                    Err(locale_err) => {
                        tracing::warn!(
                            "Failed to fetch guild locale for {}, defaulting chatbot language to id: {}",
                            guild_id,
                            locale_err
                        );
                        "id".to_string()
                    }
                }
            }
        },
        Err(e) => {
            tracing::warn!(
                "Failed to fetch user chatbot language for {}: {}, defaulting to id",
                msg.author.id,
                e
            );
            "id".to_string()
        }
    };

    let system_message = load_alya_system_message(&preferred_language);

    // Use in-memory chat history (no DB persistence)
    let history_store = CHAT_HISTORY
        .get_or_init(|| async { Arc::new(RwLock::new(HashMap::new())) })
        .await;

    // Build Gemini conversation history (system instruction is sent separately)
    let mut history_contents = Vec::new();

    let user_key = msg.author.id.to_string();
    let cutoff_time = Utc::now() - chrono::Duration::minutes(HISTORY_TTL_MINUTES);

    {
        let history_map = history_store.read().await;
        if let Some(user_history) = history_map.get(&user_key) {
            for (role, content, timestamp) in
                user_history.iter().rev().take(MAX_HISTORY_MESSAGES).rev()
            {
                if *timestamp > cutoff_time {
                    let gemini_role = if role == "assistant" { "model" } else { "user" };
                    history_contents.push(json!({
                        "role": gemini_role,
                        "parts": [{ "text": content }]
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

    let mut attachment_notes = Vec::new();
    let mut user_parts = vec![json!({ "text": user_message_content.clone() })];
    let mut inline_total_bytes: usize = 0;

    let client = reqwest::Client::new();

    for attachment in &msg.attachments {
        let filename = attachment.filename.as_str();
        let content_type = attachment
            .content_type
            .as_deref()
            .unwrap_or("application/octet-stream");

        let Some(mime_type) = normalize_attachment_mime(content_type, filename) else {
            attachment_notes.push(format!("unsupported: {} ({})", filename, content_type));
            continue;
        };

        if inline_total_bytes >= MAX_INLINE_TOTAL_BYTES {
            attachment_notes.push(format!("skipped (request size limit): {}", filename));
            continue;
        }

        match fetch_inline_attachment_base64(
            &client,
            &attachment.url,
            MAX_INLINE_FILE_BYTES,
            MAX_INLINE_TOTAL_BYTES.saturating_sub(inline_total_bytes),
        )
        .await
        {
            Ok((data, used_bytes)) => {
                user_parts.push(json!({
                    "inline_data": {
                        "mime_type": mime_type,
                        "data": data
                    }
                }));
                inline_total_bytes = inline_total_bytes.saturating_add(used_bytes);
                attachment_notes.push(format!("attached: {} ({})", filename, mime_type));
            }
            Err(err) => {
                attachment_notes.push(format!("skipped: {} ({})", filename, err));
            }
        }
    }

    let mut user_message_for_history = user_message_content.clone();
    if !attachment_notes.is_empty() {
        user_message_for_history.push_str("\n[attachments: ");
        user_message_for_history.push_str(&attachment_notes.join(", "));
        user_message_for_history.push(']');
    }

    let now = Utc::now();

    // Append current user message to Gemini payload
    history_contents.push(json!({
        "role": "user",
        "parts": user_parts
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
        "system_instruction": {
            "parts": [{ "text": system_message }]
        },
        "contents": history_contents,
        "generationConfig": {
            "temperature": 1.0
        }
    });

    let endpoint = format!(
        "https://generativelanguage.googleapis.com/v1beta/models/gemini-3-flash-preview:generateContent?key={}",
        chatbot_config.gemini_api_key
    );

    match client.post(endpoint).json(&request_body).send().await {
        Ok(response) => match response.json::<serde_json::Value>().await {
            Ok(data) => {
                if let Some(reply) = data
                    .get("candidates")
                    .and_then(|c| c.as_array())
                    .and_then(|arr| arr.first())
                    .and_then(|candidate| candidate.get("content"))
                    .and_then(|content| content.get("parts"))
                    .and_then(|parts| parts.as_array())
                {
                    let reply = reply
                        .iter()
                        .filter_map(|part| part.get("text").and_then(|text| text.as_str()))
                        .collect::<Vec<_>>()
                        .join("\n");

                    if reply.trim().is_empty() {
                        tracing::warn!("No text content in Gemini API response parts");
                        return Ok(());
                    }

                    let messages = split_message(&reply, 2000);

                    {
                        let mut history_map = history_store.write().await;
                        history_map
                            .entry(user_key.clone())
                            .or_insert_with(Vec::new)
                            .push(("assistant".to_string(), reply.clone(), Utc::now()));
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
                    tracing::warn!("No content in Gemini API response: {}", data);
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

fn normalize_attachment_mime(content_type: &str, filename: &str) -> Option<String> {
    let ct = content_type.trim().to_ascii_lowercase();
    if is_supported_gemini_mime(&ct) {
        return Some(ct);
    }

    let ext = filename
        .rsplit('.')
        .next()
        .map(|value| value.trim().to_ascii_lowercase())
        .unwrap_or_default();

    let inferred = match ext.as_str() {
        "png" => "image/png",
        "jpg" | "jpeg" => "image/jpeg",
        "webp" => "image/webp",
        "heic" => "image/heic",
        "heif" => "image/heif",
        "mp4" => "video/mp4",
        "mpeg" => "video/mpeg",
        "mov" => "video/mov",
        "avi" => "video/avi",
        "flv" => "video/x-flv",
        "mpg" => "video/mpg",
        "webm" => "video/webm",
        "wmv" => "video/wmv",
        "3gp" | "3gpp" => "video/3gpp",
        "wav" => "audio/wav",
        "mp3" => "audio/mp3",
        "aiff" => "audio/aiff",
        "aac" => "audio/aac",
        "ogg" => "audio/ogg",
        "flac" => "audio/flac",
        "pdf" => "application/pdf",
        _ => return None,
    };

    Some(inferred.to_string())
}

fn is_supported_gemini_mime(mime_type: &str) -> bool {
    matches!(
        mime_type,
        "image/png"
            | "image/jpeg"
            | "image/webp"
            | "image/heic"
            | "image/heif"
            | "video/mp4"
            | "video/mpeg"
            | "video/mov"
            | "video/avi"
            | "video/x-flv"
            | "video/mpg"
            | "video/webm"
            | "video/wmv"
            | "video/3gpp"
            | "audio/wav"
            | "audio/mp3"
            | "audio/aiff"
            | "audio/aac"
            | "audio/ogg"
            | "audio/flac"
            | "application/pdf"
    )
}

async fn fetch_inline_attachment_base64(
    client: &reqwest::Client,
    url: &str,
    max_file_bytes: usize,
    remaining_budget_bytes: usize,
) -> Result<(String, usize), String> {
    let resp = client.get(url).send().await.map_err(|e| e.to_string())?;

    if let Some(len) = resp.content_length() {
        if len as usize > max_file_bytes {
            return Err(format!("file too large: {} bytes", len));
        }
        if len as usize > remaining_budget_bytes {
            return Err(format!("request budget exceeded: {} bytes", len));
        }
    }

    let bytes = resp.bytes().await.map_err(|e| e.to_string())?;
    if bytes.len() > max_file_bytes {
        return Err(format!("file too large: {} bytes", bytes.len()));
    }
    if bytes.len() > remaining_budget_bytes {
        return Err(format!("request budget exceeded: {} bytes", bytes.len()));
    }

    Ok((general_purpose::STANDARD.encode(&bytes), bytes.len()))
}
