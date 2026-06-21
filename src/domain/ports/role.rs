use async_trait::async_trait;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum RoleError {
    #[error("LLM error")]
    LlmError,
    #[error("LLM error: {0}")]
    LlmErrorDetailed(String),
    #[error("Reasoning error")]
    ReasoningError,
}

use crate::domain::models::message::Message;

#[async_trait]
pub trait Role: Send + Sync {
    fn name(&self) -> &str {
        "Maestro AI"
    }
    fn profile(&self) -> &str;
    async fn observe(&self, messages: &[Message]) -> Result<(), RoleError>;
    async fn think(&self) -> Result<(), RoleError>;
    async fn act(&self) -> Result<Option<Message>, RoleError>;
}

#[cfg(test)]
mod tests {
    use super::*;

    struct DummyRole;

    #[async_trait]
    impl Role for DummyRole {
        fn profile(&self) -> &str {
            "dummy"
        }

        async fn observe(&self, _messages: &[Message]) -> Result<(), RoleError> {
            Ok(())
        }

        async fn think(&self) -> Result<(), RoleError> {
            Ok(())
        }

        async fn act(&self) -> Result<Option<Message>, RoleError> {
            Ok(None)
        }
    }

    fn assert_role_bounds<T>(role: T)
    where
        T: Role + Send + Sync + 'static,
    {
        let _ = role;
    }

    #[test]
    fn role_is_object_safe_and_thread_safe() {
        let role = DummyRole;
        let _trait_object: Box<dyn Role> = Box::new(DummyRole);

        assert_role_bounds(role);
    }
}
