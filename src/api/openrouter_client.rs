//! Low-level OpenRouter HTTP client for streaming chat completions.

use std::pin::Pin;

use futures::{Stream, StreamExt};
use serde::Deserialize;

use super::chat_provider::{
    ApiError, CancelToken, ChatMessage, GenerationSettings, MessageRole, SpeedMode, StreamEvent,
};
use super::http_runtime::http_client;

const OPENROUTER_CHAT_URL: &str = "https://openrouter.ai/api/v1/chat/completions";

#[derive(Debug, Deserialize)]
struct ChatCompletionChunk {
    choices: Vec<ChunkChoice>,
}

#[derive(Debug, Deserialize)]
struct ChunkChoice {
    delta: ChunkDelta,
}

#[derive(Debug, Deserialize, Default)]
struct ChunkDelta {
    #[serde(default)]
    content: Option<String>,
}

/// Streams assistant tokens from an OpenRouter chat completion request.
pub fn stream_chat_completion(
    api_key: &str,
    model: &str,
    messages: &[ChatMessage],
    generation: &GenerationSettings,
    cancel: CancelToken,
) -> Pin<Box<dyn Stream<Item = Result<StreamEvent, ApiError>> + Send>> {
    let api_key = api_key.to_string();
    let model = model.to_string();
    let messages = messages.to_vec();
    let generation = generation.clone();
    let client = http_client().clone();

    Box::pin(async_stream::stream! {
        if cancel.is_cancelled() {
            yield Err(ApiError::RequestFailed("cancelled before start".into()));
            return;
        }

        let body = build_request_body(&model, messages.as_slice(), &generation);

        let response = match client
            .post(OPENROUTER_CHAT_URL)
            .bearer_auth(&api_key)
            .header("HTTP-Referer", "https://github.com/bengidev/opencore_rustroops")
            .header("X-Title", "OpenCore Rustroops")
            .json(&body)
            .send()
            .await
        {
            Ok(response) => response,
            Err(error) => {
                yield Err(ApiError::RequestFailed(error.to_string()));
                return;
            }
        };

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            yield Err(ApiError::RequestFailed(format!("HTTP {status}: {body}")));
            return;
        }

        let mut line_buffer = String::new();
        let mut byte_tail: Vec<u8> = Vec::new();
        let mut byte_stream = response.bytes_stream();
        let mut stream_finished = false;

        while let Some(chunk) = byte_stream.next().await {
            if cancel.is_cancelled() {
                yield Err(ApiError::RequestFailed("cancelled".into()));
                return;
            }

            let chunk = match chunk {
                Ok(chunk) => chunk,
                Err(error) => {
                    yield Err(ApiError::RequestFailed(error.to_string()));
                    return;
                }
            };

            byte_tail.extend_from_slice(&chunk);
            let decoded = take_valid_utf8_prefix(&mut byte_tail);
            line_buffer.push_str(&decoded);

            while let Some(line_end) = line_buffer.find('\n') {
                let line = line_buffer.drain(..=line_end).collect::<String>();
                if let Some(event) = parse_sse_line(line.trim()) {
                    if matches!(&event, Ok(StreamEvent::Done)) {
                        stream_finished = true;
                    }
                    yield event;
                }
            }
        }

        if !byte_tail.is_empty() {
            line_buffer.push_str(&String::from_utf8_lossy(&byte_tail));
            byte_tail.clear();
        }

        let trailing = line_buffer.trim();
        if !trailing.is_empty()
            && let Some(event) = parse_sse_line(trailing)
        {
            if matches!(&event, Ok(StreamEvent::Done)) {
                stream_finished = true;
            }
            yield event;
        }

        if !stream_finished {
            yield Ok(StreamEvent::Done);
        }
    })
}

/// Decodes as much of `bytes` as forms valid UTF-8, leaving any trailing partial codepoint in place.
fn take_valid_utf8_prefix(bytes: &mut Vec<u8>) -> String {
    let mut end = bytes.len();
    while end > 0 && std::str::from_utf8(&bytes[..end]).is_err() {
        end -= 1;
    }
    let valid = bytes.drain(..end).collect::<Vec<_>>();
    String::from_utf8(valid).expect("validated utf-8 prefix")
}

fn openrouter_message(message: &ChatMessage) -> serde_json::Value {
    let role = match message.role {
        MessageRole::System => "system",
        MessageRole::User => "user",
        MessageRole::Assistant => "assistant",
    };
    serde_json::json!({
        "role": role,
        "content": message.content,
    })
}

fn build_request_body(
    model: &str,
    messages: &[ChatMessage],
    generation: &GenerationSettings,
) -> serde_json::Value {
    let mut body = serde_json::json!({
        "model": model,
        "stream": true,
        "messages": messages
            .iter()
            .filter(|message| !message.content.trim().is_empty())
            .map(openrouter_message)
            .collect::<Vec<_>>(),
    });

    if let Some(temperature) = generation.temperature {
        body["temperature"] = serde_json::json!(temperature);
    }
    if let Some(max_tokens) = generation.max_tokens {
        body["max_tokens"] = serde_json::json!(max_tokens);
    }
    if let Some(effort) = &generation.reasoning_effort {
        body["reasoning"] = serde_json::json!({ "effort": effort });
    }
    apply_speed_mode(&mut body, model, generation.speed_mode);

    body
}

fn apply_speed_mode(body: &mut serde_json::Value, model: &str, speed_mode: SpeedMode) {
    if speed_mode != SpeedMode::Fast || model.ends_with("-fast") {
        return;
    }

    if model == "anthropic/claude-opus-4.6"
        || model.starts_with("anthropic/claude-opus-4.7")
        || model.starts_with("anthropic/claude-opus-4.8")
    {
        body["speed"] = serde_json::json!("fast");
    } else if model.starts_with("openai/gpt-5")
        && (model.contains("codex") || model.starts_with("openai/gpt-5.5"))
    {
        body["service_tier"] = serde_json::json!("priority");
    }
}

fn parse_sse_line(line: &str) -> Option<Result<StreamEvent, ApiError>> {
    let data = line.strip_prefix("data: ")?.trim();
    if data == "[DONE]" {
        return Some(Ok(StreamEvent::Done));
    }

    let chunk: ChatCompletionChunk = match serde_json::from_str(data) {
        Ok(chunk) => chunk,
        Err(error) => return Some(Err(ApiError::ParseError(error.to_string()))),
    };

    chunk
        .choices
        .first()
        .and_then(|choice| choice.delta.content.clone())
        .filter(|content| !content.is_empty())
        .map(|content| Ok(StreamEvent::Token(content)))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_sse_line_extracts_token_content() {
        let line = r#"data: {"choices":[{"delta":{"content":"Hi"}}]}"#;
        let event = parse_sse_line(line).expect("event").expect("ok");
        assert_eq!(event, StreamEvent::Token("Hi".into()));
    }

    #[test]
    fn parse_sse_line_handles_done_marker() {
        let event = parse_sse_line("data: [DONE]").expect("event").expect("ok");
        assert_eq!(event, StreamEvent::Done);
    }

    #[test]
    fn build_request_body_includes_supported_generation_fields() {
        let body = build_request_body(
            "openai/gpt-4",
            &[],
            &GenerationSettings {
                temperature: Some(0.8),
                max_tokens: Some(1024),
                reasoning_effort: Some("medium".into()),
                speed_mode: SpeedMode::Normal,
            },
        );
        assert!(body.get("temperature").is_some());
        assert_eq!(body["max_tokens"], 1024);
        assert_eq!(body["reasoning"]["effort"], "medium");
    }

    #[test]
    fn build_request_body_omits_unset_generation_fields() {
        let body = build_request_body("openai/gpt-4", &[], &GenerationSettings::default());
        assert!(body.get("temperature").is_none());
        assert!(body.get("max_tokens").is_none());
        assert!(body.get("reasoning").is_none());
        assert!(body.get("speed").is_none());
        assert!(body.get("service_tier").is_none());
    }

    #[test]
    fn build_request_body_sets_anthropic_speed_for_fast_mode() {
        let body = build_request_body(
            "anthropic/claude-opus-4.8",
            &[],
            &GenerationSettings {
                speed_mode: SpeedMode::Fast,
                ..GenerationSettings::default()
            },
        );
        assert_eq!(body["speed"], "fast");
    }

    #[test]
    fn build_request_body_sets_openai_priority_for_codex_fast_mode() {
        let body = build_request_body(
            "openai/gpt-5.3-codex",
            &[],
            &GenerationSettings {
                speed_mode: SpeedMode::Fast,
                ..GenerationSettings::default()
            },
        );
        assert_eq!(body["service_tier"], "priority");
    }

    #[test]
    fn build_request_body_omits_empty_messages() {
        let body = build_request_body(
            "cohere/command",
            &[
                ChatMessage {
                    role: MessageRole::User,
                    content: "hello".into(),
                },
                ChatMessage {
                    role: MessageRole::Assistant,
                    content: String::new(),
                },
            ],
            &GenerationSettings::default(),
        );
        let messages = body["messages"].as_array().expect("messages");
        assert_eq!(messages.len(), 1);
        assert_eq!(messages[0]["content"], "hello");
    }

    #[test]
    fn take_valid_utf8_prefix_preserves_split_multibyte_sequence() {
        let snowman = "☃";
        let bytes = snowman.as_bytes();
        let mut buffer = bytes[..2].to_vec();
        assert!(take_valid_utf8_prefix(&mut buffer).is_empty());
        assert_eq!(buffer, bytes[..2]);

        buffer.extend_from_slice(&bytes[2..]);
        assert_eq!(take_valid_utf8_prefix(&mut buffer), snowman);
        assert!(buffer.is_empty());
    }
}
