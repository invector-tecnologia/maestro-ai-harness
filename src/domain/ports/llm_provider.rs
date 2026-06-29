use async_trait::async_trait;
use serde::{Deserialize, Serialize};

use crate::domain::ports::role::RoleError;

/// Request structure for LLM chat API
#[derive(Debug, Clone)]
pub struct LlmRequest {
    /// List of messages in the conversation (system, user, assistant roles)
    pub messages: Vec<LlmMessage>,
    /// Optional tools/functions the model can call
    pub tools: Option<Vec<ToolDefinition>>,
    /// Whether to stream the response
    pub stream: bool,
    /// Generation configuration (temperature, max_tokens, etc.)
    pub generation_config: GenerationConfig,
}

/// Individual message in a chat sequence
#[derive(Debug, Clone)]
pub struct LlmMessage {
    pub role: MessageRole,
    pub content: String,
}

#[derive(Debug, Clone)]
pub enum MessageRole {
    System,
    User,
    Assistant,
    Tool,
}

/// Tool/function definition for function calling
#[derive(Debug, Clone)]
pub struct ToolDefinition {
    pub name: String,
    pub description: Option<String>,
    pub input_schema: Option<String>, // JSON schema as string
}

/// Response structure from LLM chat API
#[derive(Debug, Clone)]
pub struct LlmResponse {
    /// Text content of the response (may be empty if tool calls exist)
    pub text: Option<String>,
    /// Tool calls requested by the model
    pub tool_calls: Vec<ToolCall>,
    /// Reason the model stopped generating
    pub finish_reason: String, // "stop", "tool_calls", "max_tokens", etc.
    /// Token usage statistics
    pub usage: Option<TokenUsage>,
}

#[derive(Debug, Clone)]
pub struct ToolCall {
    pub id: String,
    pub name: String,
    pub arguments: String, // JSON as string
}

#[derive(Debug, Clone)]
pub struct TokenUsage {
    pub input_tokens: usize,
    pub output_tokens: usize,
    pub total_tokens: usize,
}

/// Generation configuration for LLM requests
#[derive(Debug, Clone)]
pub struct GenerationConfig {
    pub temperature: f32,
    pub max_tokens: Option<usize>,
    pub top_p: Option<f32>,
    pub top_k: Option<i32>,
}

impl Default for GenerationConfig {
    fn default() -> Self {
        Self {
            temperature: 0.7,
            max_tokens: None,
            top_p: Some(0.9),
            top_k: None,
        }
    }
}

/// Capabilities of a provider (determines what features are available)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderCapabilities {
    pub supports_tools: bool,
    pub supports_streaming: bool,
    pub supports_json_mode: bool,
    pub supports_reasoning_controls: bool,
    pub max_context_tokens: usize,
}

impl Default for ProviderCapabilities {
    fn default() -> Self {
        Self {
            supports_tools: false,
            supports_streaming: false,
            supports_json_mode: false,
            supports_reasoning_controls: false,
            max_context_tokens: 4096,
        }
    }
}

/// Outcome of the SENSE-stage model-availability probe.
///
/// Drives the onboarding dual-engine switch: `Available` selects the LLM-driven
/// interview (Option B); any other state selects deterministic guided setup
/// (Option A) until the provider becomes `Available`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProviderStatus {
    /// Reachable, authenticated, and the configured model is being served.
    Available,
    /// The endpoint could not be reached (network/DNS/connection failure).
    Unreachable,
    /// Reached, but authentication was rejected.
    Unauthorized,
    /// Reached and authenticated, but the configured model is not listed.
    ModelMissing,
}

#[async_trait]
pub trait LlmProvider: Send + Sync {
    /// Full chat-based API with messages, tools, and streaming support
    async fn chat(&self, request: LlmRequest) -> Result<LlmResponse, RoleError>;

    /// Returns capabilities of this provider
    fn capabilities(&self) -> ProviderCapabilities;

    /// SENSE stage: probe whether this provider is reachable, authenticated, and
    /// serving its configured model.
    ///
    /// The default implementation performs a minimal completion ping and only
    /// distinguishes `Available` from `Unreachable`. Concrete adapters override
    /// this with a real health check (model catalog + auth verification).
    async fn probe(&self) -> ProviderStatus {
        match self.text_only("ping").await {
            Ok(_) => ProviderStatus::Available,
            Err(_) => ProviderStatus::Unreachable,
        }
    }

    /// Helper: text-only completion (backward-compatible wrapper around chat)
    async fn text_only(&self, prompt: &str) -> Result<String, RoleError> {
        let request = LlmRequest {
            messages: vec![LlmMessage {
                role: MessageRole::User,
                content: prompt.to_string(),
            }],
            tools: None,
            stream: false,
            generation_config: GenerationConfig::default(),
        };

        let response = self.chat(request).await?;
        response.text.ok_or(RoleError::LlmError)
    }

    /// Legacy method for backward compatibility — calls text_only()
    async fn generate_completion(&self, prompt: &str) -> Result<String, RoleError> {
        self.text_only(prompt).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;

    struct DummyLlmProvider;

    #[async_trait]
    impl LlmProvider for DummyLlmProvider {
        async fn chat(&self, request: LlmRequest) -> Result<LlmResponse, RoleError> {
            let prompt = request
                .messages
                .first()
                .map(|m| m.content.clone())
                .unwrap_or_else(|| "unknown".to_string());
            Ok(LlmResponse {
                text: Some(format!("response for: {}", prompt)),
                tool_calls: vec![],
                finish_reason: "stop".to_string(),
                usage: None,
            })
        }

        fn capabilities(&self) -> ProviderCapabilities {
            ProviderCapabilities::default()
        }
    }

    fn assert_shared_object_safe(provider: Arc<dyn LlmProvider>)
    where
        Arc<dyn LlmProvider>: Send + Sync + 'static,
    {
        let _ = provider;
    }

    #[test]
    fn llm_provider_is_object_safe_and_thread_safe() {
        let provider: Arc<dyn LlmProvider> = Arc::new(DummyLlmProvider);

        assert_shared_object_safe(provider);
    }

    #[tokio::test]
    async fn default_probe_reports_available_when_completion_succeeds() {
        let provider: Arc<dyn LlmProvider> = Arc::new(DummyLlmProvider);

        assert_eq!(provider.probe().await, ProviderStatus::Available);
    }
}
