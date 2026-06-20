use std::time::SystemTime;

use async_trait::async_trait;

/// Runtime lifecycle event emitted by agents for observability.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RuntimeEvent {
    /// Agent started observing a message
    AgentObserving {
        agent_name: String,
        message_id: String,
    },
    /// Agent entered thinking phase
    AgentThinking { agent_name: String, context: String },
    /// Agent completed thinking and is about to act
    AgentActing {
        agent_name: String,
        decision: String,
    },
    /// Agent generated output
    AgentActed {
        agent_name: String,
        output: String,
        handoff_target: Option<String>,
    },
    /// Skill execution started (user-triggered)
    SkillExecutionStarted {
        persona_name: String,
        skill_name: String,
        input: String,
    },
    /// Skill execution completed
    SkillExecutionCompleted {
        persona_name: String,
        skill_name: String,
        result: String,
        success: bool,
    },
    /// Error during execution
    ExecutionError {
        agent_name: String,
        error_message: String,
    },
}

impl RuntimeEvent {
    pub fn timestamp(&self) -> SystemTime {
        SystemTime::now()
    }

    pub fn agent_name(&self) -> String {
        match self {
            RuntimeEvent::AgentObserving { agent_name, .. } => agent_name.clone(),
            RuntimeEvent::AgentThinking { agent_name, .. } => agent_name.clone(),
            RuntimeEvent::AgentActing { agent_name, .. } => agent_name.clone(),
            RuntimeEvent::AgentActed { agent_name, .. } => agent_name.clone(),
            RuntimeEvent::SkillExecutionStarted { persona_name, .. } => persona_name.clone(),
            RuntimeEvent::SkillExecutionCompleted { persona_name, .. } => persona_name.clone(),
            RuntimeEvent::ExecutionError { agent_name, .. } => agent_name.clone(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct RuntimeEventWithTimestamp {
    pub event: RuntimeEvent,
    pub timestamp: SystemTime,
}

#[derive(Debug, thiserror::Error)]
pub enum ObserverError {
    #[error("Observador nao conseguiu processar evento")]
    ProcessingFailure,
}

/// Trait for observing agent runtime lifecycle events.
#[async_trait]
pub trait RuntimeObserver: Send + Sync {
    /// Called when a runtime event occurs.
    async fn on_event(&self, event: RuntimeEventWithTimestamp) -> Result<(), ObserverError>;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn runtime_event_extracts_agent_name() {
        let event = RuntimeEvent::AgentThinking {
            agent_name: "tester".to_string(),
            context: "test context".to_string(),
        };
        assert_eq!(event.agent_name(), "tester".to_string());
    }

    #[test]
    fn runtime_event_with_timestamp_records_time() {
        let event = RuntimeEvent::AgentObserving {
            agent_name: "observer".to_string(),
            message_id: "msg-123".to_string(),
        };
        let _with_ts = RuntimeEventWithTimestamp {
            event: event.clone(),
            timestamp: SystemTime::now(),
        };
        // Just verify it constructs
        assert_eq!(event.agent_name(), "observer".to_string());
    }
}
