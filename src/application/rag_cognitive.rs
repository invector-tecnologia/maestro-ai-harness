//! Cognitive wrapper around [`RagService`].
//!
//! Maestro models every agent on the same `observe -> think -> act` cognitive
//! cycle. Retrieval is no exception: this wrapper narrates a RAG query as the
//! same four-phase lifecycle (`AgentObserving`, `AgentThinking`, `AgentActing`,
//! `AgentActed`) through the shared [`RuntimeObserver`] channel, while
//! delegating the actual retrieval to the inner [`RagService`].
//!
//! It is intentionally non-breaking: the wrapper holds an `Arc<RagService>`,
//! adds no new retrieval behavior, and returns exactly what the service returns.
//! The observer is optional, so the wrapper can be used without any telemetry
//! wiring.

use std::sync::Arc;
use std::time::SystemTime;

use uuid::Uuid;

use crate::application::agent_observability::{
    RuntimeEvent, RuntimeEventWithTimestamp, RuntimeObserver,
};
use crate::application::rag::{RagApplicationError, RagService};
use crate::domain::models::rag::{RagAnswer, RagQuery};

/// Default cognitive identity used when narrating retrieval events.
const DEFAULT_AGENT_NAME: &str = "RAG Retriever";

/// A retrieval agent that narrates the cognitive cycle around [`RagService`].
pub struct RagCognitiveAgent {
    service: Arc<RagService>,
    observer: Option<Arc<dyn RuntimeObserver>>,
    agent_name: String,
}

impl RagCognitiveAgent {
    /// Wrap a retrieval service without any observer attached.
    pub fn new(service: Arc<RagService>) -> Self {
        Self {
            service,
            observer: None,
            agent_name: DEFAULT_AGENT_NAME.to_string(),
        }
    }

    /// Wrap a retrieval service and stream cognitive events to `observer`.
    pub fn with_observer(service: Arc<RagService>, observer: Arc<dyn RuntimeObserver>) -> Self {
        Self {
            service,
            observer: Some(observer),
            agent_name: DEFAULT_AGENT_NAME.to_string(),
        }
    }

    /// Override the cognitive identity used in emitted events.
    pub fn with_agent_name(mut self, agent_name: impl Into<String>) -> Self {
        self.agent_name = agent_name.into();
        self
    }

    /// The cognitive identity used in emitted events.
    pub fn agent_name(&self) -> &str {
        &self.agent_name
    }

    /// Borrow the wrapped retrieval service.
    pub fn service(&self) -> &Arc<RagService> {
        &self.service
    }

    async fn emit(&self, event: RuntimeEvent) {
        if let Some(observer) = &self.observer {
            let _ = observer
                .on_event(RuntimeEventWithTimestamp {
                    event,
                    timestamp: SystemTime::now(),
                })
                .await;
        }
    }

    /// Answer `question` while narrating the cognitive cycle.
    ///
    /// The retrieval result is produced solely by the wrapped [`RagService`];
    /// this method only adds observability around it.
    pub async fn query(
        &self,
        question: &str,
        top_k: usize,
    ) -> Result<RagAnswer, RagApplicationError> {
        // SENSE: register the incoming question.
        let message_id = Uuid::new_v4().to_string();
        self.emit(RuntimeEvent::AgentObserving {
            agent_name: self.agent_name.clone(),
            message_id,
        })
        .await;

        // THINK: classify the question into knowledge domains (no side effects).
        let classified = RagQuery::classify(question);
        self.emit(RuntimeEvent::AgentThinking {
            agent_name: self.agent_name.clone(),
            context: format!(
                "classified question into {} knowledge domain(s)",
                classified.domains.len()
            ),
        })
        .await;

        // ACT: delegate retrieval to the inner service.
        self.emit(RuntimeEvent::AgentActing {
            agent_name: self.agent_name.clone(),
            decision: format!("retrieving top {top_k} grounded chunk(s)"),
        })
        .await;

        let result = self.service.query(question, top_k).await;

        match &result {
            Ok(answer) => {
                self.emit(RuntimeEvent::AgentActed {
                    agent_name: self.agent_name.clone(),
                    output: format!(
                        "returned grounded answer with {} citation(s)",
                        answer.citations.len()
                    ),
                    handoff_target: None,
                })
                .await;
            }
            Err(error) => {
                self.emit(RuntimeEvent::ExecutionError {
                    agent_name: self.agent_name.clone(),
                    error_message: error.to_string(),
                })
                .await;
            }
        }

        result
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;
    use std::sync::Mutex;
    use std::time::{SystemTime, UNIX_EPOCH};

    use async_trait::async_trait;

    use crate::application::agent_observability::ObserverError;
    use crate::infrastructure::rag::local_hybrid_index::LocalHybridIndex;

    use super::*;

    #[derive(Default)]
    struct RecordingObserver {
        events: Mutex<Vec<RuntimeEvent>>,
    }

    #[async_trait]
    impl RuntimeObserver for RecordingObserver {
        async fn on_event(&self, event: RuntimeEventWithTimestamp) -> Result<(), ObserverError> {
            if let Ok(mut events) = self.events.lock() {
                events.push(event.event);
            }
            Ok(())
        }
    }

    fn unique_root() -> PathBuf {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0);
        std::env::temp_dir().join(format!("maestro-rag-cognitive-{now}"))
    }

    async fn seeded_service(root: &PathBuf) -> Arc<RagService> {
        let docs = root.join("docs");
        std::fs::create_dir_all(&docs).expect("create docs dir");
        std::fs::write(
            docs.join("kv_cache.md"),
            "KV cache prefill and decode optimization with paged attention.",
        )
        .expect("write doc");

        let index = Arc::new(LocalHybridIndex::new(root));
        let service = Arc::new(RagService::new_with_options(
            index.clone(),
            index.clone(),
            index.clone(),
            None,
            root.join("maestro").join("rag"),
        ));
        service
            .ingest_paths(vec![docs], 512)
            .await
            .expect("ingest corpus");
        service
    }

    #[tokio::test]
    async fn narrates_cognitive_cycle_and_preserves_service_answer() {
        let root = unique_root();
        let service = seeded_service(&root).await;

        // Baseline answer straight from the service.
        let expected = service
            .query("How to optimize kv cache?", 3)
            .await
            .expect("service answers");

        let observer = Arc::new(RecordingObserver::default());
        let agent = RagCognitiveAgent::with_observer(service.clone(), observer.clone());

        let wrapped = agent
            .query("How to optimize kv cache?", 3)
            .await
            .expect("agent answers");

        // Regression guard: the wrapper must not alter retrieval output.
        assert_eq!(wrapped, expected);

        let events = observer.events.lock().expect("lock events").clone();
        assert!(matches!(
            events.first(),
            Some(RuntimeEvent::AgentObserving { .. })
        ));
        assert!(events
            .iter()
            .any(|event| matches!(event, RuntimeEvent::AgentThinking { .. })));
        assert!(events
            .iter()
            .any(|event| matches!(event, RuntimeEvent::AgentActing { .. })));
        assert!(matches!(
            events.last(),
            Some(RuntimeEvent::AgentActed { .. })
        ));

        let _ = std::fs::remove_dir_all(root);
    }

    #[tokio::test]
    async fn query_without_observer_returns_answer() {
        let root = unique_root();
        let service = seeded_service(&root).await;

        let agent = RagCognitiveAgent::new(service);
        assert_eq!(agent.agent_name(), "RAG Retriever");

        let answer = agent
            .query("How to optimize kv cache?", 3)
            .await
            .expect("agent answers without observer");
        assert!(!answer.citations.is_empty());

        let _ = std::fs::remove_dir_all(root);
    }
}
