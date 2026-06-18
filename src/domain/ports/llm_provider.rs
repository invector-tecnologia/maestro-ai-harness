use async_trait::async_trait;

use crate::domain::ports::role::RoleError;

#[async_trait]
pub trait LlmProvider: Send + Sync {
    async fn generate_completion(&self, prompt: &str) -> Result<String, RoleError>;
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;

    struct DummyLlmProvider;

    #[async_trait]
    impl LlmProvider for DummyLlmProvider {
        async fn generate_completion(&self, prompt: &str) -> Result<String, RoleError> {
            Ok(format!("completion for: {prompt}"))
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
}
