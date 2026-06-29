//! Low-level OpenRouter HTTP client for streaming chat completions.

use std::pin::Pin;

use futures::{Stream, StreamExt};
use serde::Deserialize;

use super::chat_provider::{ApiError, CancelToken, ChatMessage, MessageRole, StreamEvent};
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
    cancel: CancelToken,
) -> Pin<Box<dyn Stream<Item = Result<StreamEvent, ApiError>> + Send>> {
    let api_key = api_key.to_string();
    let model = model.to_string();
    let messages = messages.to_vec();
    let client = http_client().clone();

    Box::pin(async_stream::stream! {
        if cancel.is_cancelled() {
            yield Err(ApiError::RequestFailed("cancelled before start".into()));
            return;
        }

        let body = serde_json::json!({
            "model": model,
            "stream": true,
            "messages": messages.iter().map(openrouter_message).collect::<Vec<_>>(),
        });

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
