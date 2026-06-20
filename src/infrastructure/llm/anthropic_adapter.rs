use std::sync::Arc;
use std::time::Duration;

use async_trait::async_trait;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use tracing::{error, info};

use crate::application::config::{AuthMode, ProviderConfig};
use crate::domain::ports::llm_provider::{
    LlmProvider, LlmRequest, LlmResponse, MessageRole, ProviderCapabilities,
};
use crate::domain::ports::role::RoleError;
use crate::infrastructure::llm::provider_registry::ProviderRegistryError;

pub struct AnthropicAdapter {
    client: Client,
    endpoint: String,
    model: String,
    bearer_token: Option<String>,
    max_context_tokens: usize,
}

impl AnthropicAdapter {
    pub fn from_provider_config(
        provider: &ProviderConfig,
    ) -> Result<Arc<dyn LlmProvider>, ProviderRegistryError> {
        let model = provider
            .models
            .first()
            .map(|m| m.name.clone())
            .ok_or_else(|| {
                ProviderRegistryError::InconsistentConfig(
                    "Anthropic provider has no configured models".to_string(),
                )
            })?;

        let timeout = Duration::from_millis(provider.timeout_ms);
        let client = Client::builder().timeout(timeout).build().map_err(|_| {
            ProviderRegistryError::InconsistentConfig(
                "Failed to build HTTP client for anthropic provider".to_string(),
            )
        })?;

        let bearer_token = match provider.auth_mode {
            AuthMode::None => None,
            AuthMode::Bearer => provider
                .auth_env_var
                .as_ref()
                .and_then(|var| std::env::var(var).ok()),
            AuthMode::Browser => {
                return Err(ProviderRegistryError::InconsistentConfig(
                    "Anthropic provider does not support auth_mode=browser".to_string(),
                ));
            }
        };

        let max_context_tokens = provider
            .models
            .first()
            .map(|m| m.context_window)
            .unwrap_or(200000);

        Ok(Arc::new(Self {
            client,
            endpoint: format!("{}/v1/messages", provider.endpoint.trim_end_matches('/')),
            model,
            bearer_token,
            max_context_tokens,
        }))
    }
}

/// Anthropic Messages API request format
#[derive(Debug, Serialize)]
struct MessagesRequest {
    model: String,
    messages: Vec<AnthropicMessage>,
    #[serde(skip_serializing_if = "Option::is_none")]
    system: Option<String>,
    max_tokens: u32,
}

#[derive(Debug, Serialize)]
struct AnthropicMessage {
    role: String,
    content: String,
}

#[derive(Debug, Deserialize)]
struct MessagesResponse {
    content: Vec<ContentBlock>,
    stop_reason: Option<String>,
    usage: Option<AnthropicUsage>,
}

#[derive(Debug, Deserialize)]
struct ContentBlock {
    #[serde(rename = "type")]
    content_type: String,
    text: Option<String>,
}

#[derive(Debug, Deserialize)]
struct AnthropicUsage {
    input_tokens: usize,
    output_tokens: usize,
}

#[async_trait]
impl LlmProvider for AnthropicAdapter {
    async fn chat(&self, request: LlmRequest) -> Result<LlmResponse, RoleError> {
        let started_at = std::time::Instant::now();

        // Anthropic separates system prompt from conversation messages
        let system = request
            .messages
            .iter()
            .find(|m| matches!(m.role, MessageRole::System))
            .map(|m| m.content.clone());

        let messages: Vec<AnthropicMessage> = request
            .messages
            .into_iter()
            .filter(|m| !matches!(m.role, MessageRole::System))
            .map(|m| AnthropicMessage {
                role: match m.role {
                    MessageRole::User | MessageRole::Tool => "user".to_string(),
                    MessageRole::Assistant => "assistant".to_string(),
                    MessageRole::System => "user".to_string(), // filtered above
                },
                content: m.content,
            })
            .collect();

        let max_tokens = request.generation_config.max_tokens.unwrap_or(4096) as u32;

        let api_request = MessagesRequest {
            model: self.model.clone(),
            messages,
            system,
            max_tokens,
        };

        let mut builder = self.client.post(&self.endpoint).json(&api_request);
        if let Some(token) = &self.bearer_token {
            builder = builder
                .header("x-api-key", token)
                .header("anthropic-version", "2023-06-01");
        }

        let response = builder.send().await.map_err(|error| {
            error!(latency_ms = started_at.elapsed().as_millis(), error = %error, "request failed for anthropic provider");
            RoleError::LlmError
        })?;

        let status = response.status();
        if !status.is_success() {
            error!(latency_ms = started_at.elapsed().as_millis(), status = %status, "invalid HTTP response from anthropic provider");
            return Err(RoleError::LlmError);
        }

        let payload: MessagesResponse = response.json().await.map_err(|error| {
            error!(latency_ms = started_at.elapsed().as_millis(), error = %error, "invalid payload received from anthropic provider");
            RoleError::LlmError
        })?;

        let text = payload
            .content
            .into_iter()
            .find(|b| b.content_type == "text")
            .and_then(|b| b.text)
            .map(|t| t.trim().to_string());

        let finish_reason = payload
            .stop_reason
            .unwrap_or_else(|| "end_turn".to_string());

        let usage = payload.usage.map(|u| {
            let total = u.input_tokens + u.output_tokens;
            crate::domain::ports::llm_provider::TokenUsage {
                input_tokens: u.input_tokens,
                output_tokens: u.output_tokens,
                total_tokens: total,
            }
        });

        info!(
            latency_ms = started_at.elapsed().as_millis(),
            "completion generated successfully by anthropic provider"
        );

        Ok(LlmResponse {
            text,
            tool_calls: vec![],
            finish_reason,
            usage,
        })
    }

    fn capabilities(&self) -> ProviderCapabilities {
        ProviderCapabilities {
            supports_tools: true,
            supports_streaming: true,
            supports_json_mode: false,
            supports_reasoning_controls: false,
            max_context_tokens: self.max_context_tokens,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    use tokio::net::TcpListener;

    async fn spawn_single_response_server(response: String) -> Result<String, std::io::Error> {
        let listener = TcpListener::bind("127.0.0.1:0").await?;
        let addr = listener.local_addr()?;

        tokio::spawn(async move {
            if let Ok((mut socket, _)) = listener.accept().await {
                let mut buf = [0_u8; 2048];
                let _ = socket.read(&mut buf).await;
                let _ = socket.write_all(response.as_bytes()).await;
                let _ = socket.flush().await;
            }
        });

        Ok(format!("http://{addr}"))
    }

    #[tokio::test]
    async fn returns_completion_when_payload_is_valid() {
        let body = r#"{"content":[{"type":"text","text":"pong"}],"stop_reason":"end_turn"}"#;
        let response = format!(
            "HTTP/1.1 200 OK\r\ncontent-type: application/json\r\ncontent-length: {}\r\nconnection: close\r\n\r\n{}",
            body.len(),
            body
        );

        let endpoint_result = spawn_single_response_server(response).await;
        assert!(endpoint_result.is_ok());

        if let Ok(endpoint) = endpoint_result {
            let timeout = Duration::from_millis(100);
            let client = Client::builder().timeout(timeout).build().unwrap();
            let adapter = AnthropicAdapter {
                client,
                endpoint: format!("{}/v1/messages", endpoint),
                model: "claude-3-sonnet".to_string(),
                bearer_token: None,
                max_context_tokens: 200000,
            };

            let result = adapter.text_only("ping").await;
            assert!(matches!(result, Ok(ref text) if text == "pong"));
        }
    }

    #[test]
    fn capabilities_include_tools_and_streaming_but_not_json_mode() {
        let timeout = Duration::from_millis(100);
        let client = Client::builder().timeout(timeout).build().unwrap();
        let adapter = AnthropicAdapter {
            client,
            endpoint: "https://api.anthropic.com/v1/messages".to_string(),
            model: "claude-3-opus".to_string(),
            bearer_token: None,
            max_context_tokens: 200000,
        };

        let caps = adapter.capabilities();
        assert!(caps.supports_tools);
        assert!(caps.supports_streaming);
        assert!(!caps.supports_json_mode);
        assert!(!caps.supports_reasoning_controls);
        assert_eq!(caps.max_context_tokens, 200000);
    }
}
