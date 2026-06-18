use std::sync::Arc;

use thiserror::Error;
use tokio::sync::{broadcast, RwLock};

use crate::domain::models::message::Message;

#[derive(Debug, Error, PartialEq, Eq)]
pub enum EnvironmentError {
    #[error("Nao ha assinantes ativos no barramento")]
    NoSubscribers,
}

pub struct Environment {
    history: Arc<RwLock<Vec<Message>>>,
    tx: broadcast::Sender<Message>,
}

impl Environment {
    pub fn new(capacity: usize) -> Self {
        let (tx, _) = broadcast::channel(capacity);

        Self {
            history: Arc::new(RwLock::new(Vec::new())),
            tx,
        }
    }

    pub async fn publish(&self, msg: Message) -> Result<(), EnvironmentError> {
        let mut guard = self.history.write().await;
        guard.push(msg.clone());
        drop(guard);

        self.tx
            .send(msg)
            .map(|_| ())
            .map_err(|_| EnvironmentError::NoSubscribers)
    }

    pub fn subscribe(&self) -> broadcast::Receiver<Message> {
        self.tx.subscribe()
    }

    pub async fn get_history(&self) -> Vec<Message> {
        let guard = self.history.read().await;
        guard.clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn broadcast_reaches_multiple_subscribers_and_updates_history() {
        let environment = Environment::new(16);
        let mut receiver_one = environment.subscribe();
        let mut receiver_two = environment.subscribe();
        let message = Message::new("agent-a".to_string(), "hello".to_string(), None);
        let expected = message.clone();

        let first_task = tokio::spawn(async move { receiver_one.recv().await });

        let second_task = tokio::spawn(async move { receiver_two.recv().await });

        let publish_result = environment.publish(message).await;

        let first_received = first_task.await;
        let second_received = second_task.await;
        let history = environment.get_history().await;

        assert!(publish_result.is_ok());
        assert!(matches!(first_received, Ok(Ok(ref received)) if received == &expected));
        assert!(matches!(second_received, Ok(Ok(ref received)) if received == &expected));
        assert_eq!(history, vec![expected]);
    }

    #[tokio::test]
    async fn publish_returns_error_when_no_subscribers_and_keeps_audit_history() {
        let environment = Environment::new(16);
        let message = Message::new("agent-b".to_string(), "audit-only".to_string(), None);
        let expected = message.clone();

        let publish_result = environment.publish(message).await;
        let history = environment.get_history().await;

        assert_eq!(publish_result, Err(EnvironmentError::NoSubscribers));
        assert_eq!(history, vec![expected]);
    }
}
