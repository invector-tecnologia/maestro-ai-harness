use std::sync::Arc;
use std::time::Duration;

use async_trait::async_trait;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use tracing::{error, info};

use crate::application::config::{AuthMode, ProviderConfig};
use crate::domain::ports::llm_provider::{
    LlmProvider, LlmRequest, LlmResponse, MessageRole, ProviderCapabilities, ProviderStatus,
};
use crate::domain::ports::role::RoleError;
use crate::infrastructure::llm::endpoint_utils::{
    model_in_catalog, normalize_chat_completions_endpoint, openai_models_endpoint,
};
use crate::infrastructure::llm::provider_registry::ProviderRegistryError;

pub struct OpenAiAdapter {
    client: Client,
    endpoint: String,
    model: String,
    bearer_token: Option<String>,
    max_context_tokens: usize,
}

impl OpenAiAdapter {
    pub fn from_provider_config(
        provider: &ProviderConfig,
        model: &str,
    ) -> Result<Arc<dyn LlmProvider>, ProviderRegistryError> {
        let model_spec = provider
            .models
            .iter()
            .find(|m| m.name == model)
            .ok_or_else(|| ProviderRegistryError::ModelNotConfigured {
                provider: provider.kind.clone(),
                model: model.to_string(),
            })?;

        let timeout = Duration::from_millis(provider.timeout_ms);
        let client = Client::builder().timeout(timeout).build().map_err(|_| {
            ProviderRegistryError::InconsistentConfig(
                "Failed to build HTTP client for openai provider".to_string(),
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
                    "OpenAI provider does not support auth_mode=browser".to_string(),
                ));
            }
        };

        let max_context_tokens = model_spec.context_window;

        Ok(Arc::new(Self {
            client,
            endpoint: normalize_chat_completions_endpoint(&provider.endpoint),
            model: model.to_string(),
            bearer_token,
            max_context_tokens,
        }))
    }
}

#[derive(Debug, Serialize)]
struct ChatCompletionRequest {
    model: String,
    messages: Vec<ChatMessage>,
}

#[derive(Debug, Serialize)]
struct ChatMessage {
    role: String,
    content: String,
}

#[derive(Debug, Deserialize)]
struct ChatCompletionResponse {
    choices: Vec<Choice>,
    #[serde(default)]
    usage: Option<UsageData>,
}

#[derive(Debug, Deserialize)]
struct Choice {
    message: AssistantMessage,
    finish_reason: Option<String>,
}

#[derive(Debug, Deserialize)]
struct AssistantMessage {
    content: Option<String>,
}

#[derive(Debug, Deserialize)]
struct UsageData {
    prompt_tokens: usize,
    completion_tokens: usize,
    total_tokens: usize,
}

#[derive(Debug, Deserialize)]
struct ModelsResponse {
    #[serde(default)]
    data: Vec<ModelEntry>,
}

#[derive(Debug, Deserialize)]
struct ModelEntry {
    #[serde(default)]
    id: String,
}

#[async_trait]
impl LlmProvider for OpenAiAdapter {
    async fn chat(&self, request: LlmRequest) -> Result<LlmResponse, RoleError> {
        let started_at = std::time::Instant::now();

        let messages: Vec<ChatMessage> = request
            .messages
            .into_iter()
            .map(|m| ChatMessage {
                role: match m.role {
                    MessageRole::System => "system".to_string(),
                    MessageRole::User => "user".to_string(),
                    MessageRole::Assistant => "assistant".to_string(),
                    MessageRole::Tool => "tool".to_string(),
                },
                content: m.content,
            })
            .collect();

        let api_request = ChatCompletionRequest {
            model: self.model.clone(),
            messages,
        };

        let mut builder = self.client.post(&self.endpoint).json(&api_request);
        if let Some(token) = &self.bearer_token {
            builder = builder.bearer_auth(token);
        }

        let response = builder.send().await.map_err(|error| {
            error!(latency_ms = started_at.elapsed().as_millis(), error = %error, "request failed for openai provider");
            RoleError::LlmError
        })?;

        let status = response.status();
        if !status.is_success() {
            error!(latency_ms = started_at.elapsed().as_millis(), status = %status, "invalid HTTP response from openai provider");
            return Err(RoleError::LlmError);
        }

        let payload: ChatCompletionResponse = response.json().await.map_err(|error| {
            error!(latency_ms = started_at.elapsed().as_millis(), error = %error, "invalid payload received from openai provider");
            RoleError::LlmError
        })?;

        let choice = payload.choices.into_iter().next().ok_or_else(|| {
            error!("empty choices from openai provider");
            RoleError::LlmError
        })?;

        let finish_reason = choice.finish_reason.unwrap_or_else(|| "stop".to_string());
        let text = choice.message.content.map(|t| t.trim().to_string());

        let usage = payload
            .usage
            .map(|u| crate::domain::ports::llm_provider::TokenUsage {
                input_tokens: u.prompt_tokens,
                output_tokens: u.completion_tokens,
                total_tokens: u.total_tokens,
            });

        info!(
            latency_ms = started_at.elapsed().as_millis(),
            "completion generated successfully by openai provider"
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
            supports_json_mode: true,
            supports_reasoning_controls: true,
            max_context_tokens: self.max_context_tokens,
        }
    }

    async fn probe(&self) -> ProviderStatus {
        let url = openai_models_endpoint(&self.endpoint);
        let mut request = self.client.get(&url);
        if let Some(token) = &self.bearer_token {
            request = request.bearer_auth(token);
        }

        let response = match request.send().await {
            Ok(response) => response,
            Err(_) => return ProviderStatus::Unreachable,
        };

        let status = response.status();
        if status == reqwest::StatusCode::UNAUTHORIZED || status == reqwest::StatusCode::FORBIDDEN {
            return ProviderStatus::Unauthorized;
        }
        if !status.is_success() {
            return ProviderStatus::Unreachable;
        }

        let models: ModelsResponse = match response.json().await {
            Ok(models) => models,
            Err(_) => return ProviderStatus::Unreachable,
        };

        let ids = models.data.iter().map(|m| m.id.as_str());
        if model_in_catalog(&self.model, ids) {
            ProviderStatus::Available
        } else {
            ProviderStatus::ModelMissing
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
        let body = r#"{"choices":[{"message":{"content":"pong"},"finish_reason":"stop"}]}"#;
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
            let adapter = OpenAiAdapter {
                client,
                endpoint: normalize_chat_completions_endpoint(endpoint.as_str()),
                model: "gpt-4".to_string(),
                bearer_token: None,
                max_context_tokens: 8192,
            };

            let result = adapter.text_only("ping").await;
            assert!(matches!(result, Ok(ref text) if text == "pong"));
        }
    }

    #[test]
    fn capabilities_include_tools_streaming_json_mode() {
        let timeout = Duration::from_millis(100);
        let client = Client::builder().timeout(timeout).build().unwrap();
        let adapter = OpenAiAdapter {
            client,
            endpoint: "http://localhost/v1/chat/completions".to_string(),
            model: "gpt-4".to_string(),
            bearer_token: None,
            max_context_tokens: 128000,
        };

        let caps = adapter.capabilities();
        assert!(caps.supports_tools);
        assert!(caps.supports_streaming);
        assert!(caps.supports_json_mode);
        assert!(caps.supports_reasoning_controls);
        assert_eq!(caps.max_context_tokens, 128000);
    }
}
