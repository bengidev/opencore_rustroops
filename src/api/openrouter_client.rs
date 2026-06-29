//! Low-level OpenRouter HTTP client for streaming chat completions.

use std::pin::Pin;

use futures::{Stream, StreamExt};
use reqwest::Client;
use serde::Deserialize;

use super::chat_provider::{ApiError, CancelToken, ChatMessage, MessageRole, StreamEvent};

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

    Box::pin(async_stream::stream! {
        if cancel.is_cancelled() {
            yield Err(ApiError::RequestFailed("cancelled before start".into()));
            return;
        }

        let client = match Client::builder().build() {
            Ok(client) => client,
            Err(error) => {
                yield Err(ApiError::RequestFailed(error.to_string()));
                return;
            }
        };

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

        let mut buffer = String::new();
        let mut byte_stream = response.bytes_stream();

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

            buffer.push_str(&String::from_utf8_lossy(&chunk));
            while let Some(line_end) = buffer.find('\n') {
                let line = buffer.drain(..=line_end).collect::<String>();
                if let Some(event) = parse_sse_line(line.trim()) {
                    yield event;
                }
            }
        }

        let trailing = buffer.trim();
        if !trailing.is_empty() {
            if let Some(event) = parse_sse_line(trailing) {
                yield event;
            }
        }

        yield Ok(StreamEvent::Done);
    })
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
}
