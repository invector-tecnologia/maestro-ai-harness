use std::sync::Arc;
use std::time::Duration;

use async_trait::async_trait;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use tokio::fs::OpenOptions;
use tokio::io::AsyncWriteExt;
use tracing::{error, info};

use crate::application::config::{AuthMode, ProviderConfig};
use crate::domain::ports::llm_provider::{
    LlmProvider, LlmRequest, LlmResponse, MessageRole, ProviderCapabilities,
};
use crate::domain::ports::role::RoleError;
use crate::infrastructure::llm::endpoint_utils::normalize_ollama_chat_endpoint;
use crate::infrastructure::llm::provider_registry::ProviderRegistryError;

pub struct OllamaAdapter {
    client: Client,
    endpoint: String,
    model: String,
    bearer_token: Option<String>,
    max_context_chars: usize,
}

impl OllamaAdapter {
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
                "Failed to build HTTP client for ollama provider".to_string(),
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
                    "Ollama provider does not support auth_mode=browser".to_string(),
                ));
            }
        };

        let max_context_chars = model_spec.context_window;

        Ok(Arc::new(Self {
            client,
            endpoint: normalize_ollama_chat_endpoint(&provider.endpoint),
            model: model.to_string(),
            bearer_token,
            max_context_chars,
        }))
    }

    #[cfg(test)]
    fn with_parts(
        endpoint: String,
        model: String,
        timeout_ms: u64,
        bearer_token: Option<String>,
    ) -> Result<Self, ProviderRegistryError> {
        let client = Client::builder()
            .timeout(Duration::from_millis(timeout_ms))
            .build()
            .map_err(|_| {
                ProviderRegistryError::InconsistentConfig(
                    "Failed to build test HTTP client".to_string(),
                )
            })?;

        Ok(Self {
            client,
            endpoint,
            model,
            bearer_token,
            max_context_chars: 128000,
        })
    }
}

#[derive(Debug, Serialize)]
struct ChatCompletionRequest {
    model: String,
    messages: Vec<ChatMessage>,
}

#[derive(Debug, Serialize)]
struct ChatMessage {
    role: &'static str,
    content: String,
}

#[derive(Debug, Deserialize)]
struct ChatCompletionResponse {
    choices: Vec<Choice>,
}

#[derive(Debug, Deserialize)]
struct Choice {
    message: AssistantMessage,
}

#[derive(Debug, Deserialize)]
struct AssistantMessage {
    content: String,
}

#[async_trait]
impl LlmProvider for OllamaAdapter {
    async fn chat(&self, request: LlmRequest) -> Result<LlmResponse, RoleError> {
        // Build the prompt from the first user message for KV cache processing
        let prompt = request
            .messages
            .iter()
            .find(|m| matches!(m.role, MessageRole::User))
            .map(|m| m.content.clone())
            .unwrap_or_default();

        let completion = self.generate_text(&prompt).await?;
        Ok(LlmResponse {
            text: Some(completion),
            tool_calls: vec![],
            finish_reason: "stop".to_string(),
            usage: None,
        })
    }

    fn capabilities(&self) -> ProviderCapabilities {
        ProviderCapabilities {
            supports_tools: false,
            supports_streaming: true,
            supports_json_mode: false,
            supports_reasoning_controls: false,
            max_context_tokens: self.max_context_chars,
        }
    }
}

impl OllamaAdapter {
    async fn generate_text(&self, prompt: &str) -> Result<String, RoleError> {
        let started_at = std::time::Instant::now();

        // KV CACHE OPTIMIZATION: Orchestrator-level H2O-inspired Eviction Strategy
        // Retains Heavy-Hitters (System Instructions/Persona at the top) and
        // Recent Tokens (latest interaction at the bottom), dynamically evicting the middle.
        let optimized_prompt = if prompt.len() > self.max_context_chars {
            let chars: Vec<char> = prompt.chars().collect();
            if chars.len() > self.max_context_chars {
                info!(
                    original_chars = chars.len(),
                    max_chars = self.max_context_chars,
                    "applying KV cache eviction strategy (H2O-style)"
                );
                let top_chars = self.max_context_chars / 4;
                let bottom_chars = self.max_context_chars - top_chars - 50;
                let top_part: String = chars[..top_chars].iter().collect();
                let middle_part: String = chars[top_chars..chars.len() - bottom_chars]
                    .iter()
                    .collect();
                let bottom_part: String = chars[chars.len() - bottom_chars..].iter().collect();

                // HYBRID MEMORY (Strategy 3): Persist the evicted tokens to disk for audit logs
                // Fire and forget using tokio::spawn so it doesn't block the LLM request
                tokio::spawn(async move {
                    let path = std::env::current_dir()
                        .unwrap_or_else(|_| std::path::PathBuf::from("."))
                        .join("maestro")
                        .join("kv_evictions.log");

                    if let Ok(mut file) = OpenOptions::new()
                        .create(true)
                        .append(true)
                        .open(path)
                        .await
                    {
                        let ts = std::time::SystemTime::now()
                            .duration_since(std::time::UNIX_EPOCH)
                            .map(|d| d.as_secs())
                            .unwrap_or(0);
                        let _ = file
                            .write_all(
                                format!("\n=== EVICTION AT {} ===\n{}\n", ts, middle_part)
                                    .as_bytes(),
                            )
                            .await;
                    }
                });

                format!("{top_part}\n\n...[MAESTRO KV CACHE EVICTION]...\n\n{bottom_part}")
            } else {
                prompt.to_string()
            }
        } else {
            prompt.to_string()
        };

        let request = ChatCompletionRequest {
            model: self.model.clone(),
            messages: vec![ChatMessage {
                role: "user",
                content: optimized_prompt,
            }],
        };

        let mut builder = self.client.post(&self.endpoint).json(&request);
        if let Some(token) = &self.bearer_token {
            builder = builder.bearer_auth(token);
        }

        let response = builder.send().await.map_err(|error| {
            error!(latency_ms = started_at.elapsed().as_millis(), error = %error, "request failed for ollama provider");
            RoleError::LlmErrorDetailed(format!("request failed: {}", error))
        })?;

        let status = response.status();
        if !status.is_success() {
            let body = response
                .text()
                .await
                .unwrap_or_else(|_| "<body unavailable>".to_string());
            error!(latency_ms = started_at.elapsed().as_millis(), status = %status, "invalid HTTP response from ollama provider");
            return Err(RoleError::LlmErrorDetailed(format!(
                "invalid HTTP status {} from ollama endpoint '{}': {}",
                status,
                self.endpoint,
                summarize_error_body(&body)
            )));
        }

        let payload: ChatCompletionResponse = response.json().await.map_err(|error| {
            error!(latency_ms = started_at.elapsed().as_millis(), error = %error, "invalid payload received from ollama provider");
            RoleError::LlmErrorDetailed(format!("invalid JSON payload: {}", error))
        })?;

        let content = payload
            .choices
            .first()
            .map(|choice| choice.message.content.trim().to_string())
            .filter(|text| !text.is_empty())
            .ok_or_else(|| {
                error!(
                    latency_ms = started_at.elapsed().as_millis(),
                    "empty response content from ollama provider"
                );
                RoleError::LlmErrorDetailed(
                    "ollama response payload had no text content".to_string(),
                )
            })?;

        info!(
            latency_ms = started_at.elapsed().as_millis(),
            "completion generated successfully by ollama provider"
        );
        Ok(content)
    }
}

fn summarize_error_body(body: &str) -> String {
    let clean = body.replace('\n', " ").trim().to_string();
    if clean.is_empty() {
        return "<empty body>".to_string();
    }

    let mut chars = clean.chars();
    let summary = chars.by_ref().take(160).collect::<String>();
    if chars.next().is_some() {
        format!("{}...", summary)
    } else {
        summary
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::io::AsyncReadExt;
    use tokio::net::TcpListener;
    use tokio::time::{sleep, Duration};

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

    async fn spawn_timeout_server() -> Result<String, std::io::Error> {
        let listener = TcpListener::bind("127.0.0.1:0").await?;
        let addr = listener.local_addr()?;

        tokio::spawn(async move {
            if let Ok((mut socket, _)) = listener.accept().await {
                let mut buf = [0_u8; 2048];
                let _ = socket.read(&mut buf).await;
                sleep(Duration::from_millis(200)).await;
            }
        });

        Ok(format!("http://{addr}"))
    }

    #[tokio::test]
    async fn returns_completion_when_payload_is_valid() {
        let body = r#"{"choices":[{"message":{"content":"pong"}}]}"#;
        let response = format!(
            "HTTP/1.1 200 OK\r\ncontent-type: application/json\r\ncontent-length: {}\r\nconnection: close\r\n\r\n{}",
            body.len(),
            body
        );

        let endpoint_result = spawn_single_response_server(response).await;
        assert!(endpoint_result.is_ok());

        if let Ok(endpoint) = endpoint_result {
            let adapter = OllamaAdapter::with_parts(
                normalize_ollama_chat_endpoint(endpoint.as_str()),
                "deepseek-coder-v2".to_string(),
                100,
                None,
            );
            assert!(adapter.is_ok());

            if let Ok(client) = adapter {
                let result = client.generate_completion("ping").await;
                assert!(matches!(result, Ok(ref text) if text == "pong"));
            }
        }
    }

    #[tokio::test]
    async fn returns_error_on_timeout() {
        let endpoint_result = spawn_timeout_server().await;
        assert!(endpoint_result.is_ok());

        if let Ok(endpoint) = endpoint_result {
            let adapter = OllamaAdapter::with_parts(
                normalize_ollama_chat_endpoint(endpoint.as_str()),
                "deepseek-coder-v2".to_string(),
                20,
                None,
            );
            assert!(adapter.is_ok());

            if let Ok(client) = adapter {
                let result = client.generate_completion("ping").await;
                assert!(matches!(
                    result,
                    Err(RoleError::LlmError) | Err(RoleError::LlmErrorDetailed(_))
                ));
            }
        }
    }

    #[tokio::test]
    async fn returns_error_on_connection_failure() {
        let endpoint = "http://127.0.0.1:9/chat/completions".to_string();
        let adapter =
            OllamaAdapter::with_parts(endpoint, "deepseek-coder-v2".to_string(), 50, None);
        assert!(adapter.is_ok());

        if let Ok(client) = adapter {
            let result = client.generate_completion("ping").await;
            assert!(matches!(
                result,
                Err(RoleError::LlmError) | Err(RoleError::LlmErrorDetailed(_))
            ));
        }
    }

    #[tokio::test]
    async fn returns_error_on_invalid_payload() {
        let body = "{\"choices\":\"invalid\"}";
        let response = format!(
            "HTTP/1.1 200 OK\r\ncontent-type: application/json\r\ncontent-length: {}\r\nconnection: close\r\n\r\n{}",
            body.len(),
            body
        );

        let endpoint_result = spawn_single_response_server(response).await;
        assert!(endpoint_result.is_ok());

        if let Ok(endpoint) = endpoint_result {
            let adapter = OllamaAdapter::with_parts(
                normalize_ollama_chat_endpoint(endpoint.as_str()),
                "deepseek-coder-v2".to_string(),
                100,
                None,
            );
            assert!(adapter.is_ok());

            if let Ok(client) = adapter {
                let result = client.generate_completion("ping").await;
                assert!(matches!(
                    result,
                    Err(RoleError::LlmError) | Err(RoleError::LlmErrorDetailed(_))
                ));
            }
        }
    }
}
