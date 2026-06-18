use std::clone::Clone;
use std::cmp::Eq;
use std::cmp::PartialEq;
use std::fmt::Debug;
use uuid::Uuid;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Message {
    id: Uuid,
    sender: String,
    content: String,
    cause_by: Option<Uuid>,
}

impl Message {
    pub fn new(sender: String, content: String, cause_by: Option<Uuid>) -> Self {
        let id = Uuid::new_v4();
        Self {
            id,
            sender,
            content,
            cause_by,
        }
    }

    pub fn id(&self) -> Uuid {
        self.id
    }

    pub fn sender(&self) -> &str {
        &self.sender
    }

    pub fn content(&self) -> &str {
        &self.content
    }

    pub fn cause_by(&self) -> Option<Uuid> {
        self.cause_by
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pass_through(message: Message) -> Message {
        message
    }

    #[test]
    fn new_sets_all_fields_when_cause_by_is_present() {
        let cause_by = Some(Uuid::new_v4());
        let message = Message::new("agent-a".to_string(), "hello".to_string(), cause_by);

        assert_eq!(message.sender, "agent-a");
        assert_eq!(message.content, "hello");
        assert_eq!(message.cause_by, cause_by);
    }

    #[test]
    fn new_supports_missing_cause_by() {
        let message = Message::new("agent-b".to_string(), "world".to_string(), None);

        assert_eq!(message.sender, "agent-b");
        assert_eq!(message.content, "world");
        assert_eq!(message.cause_by, None);
    }

    #[test]
    fn clone_and_pass_through_preserve_message_data() {
        let cause_by = Some(Uuid::new_v4());
        let original = Message::new("agent-c".to_string(), "same".to_string(), cause_by);
        let cloned = original.clone();
        let moved = pass_through(cloned.clone());

        assert_eq!(original, cloned);
        assert_eq!(cloned, moved);
        assert_eq!(moved.sender, "agent-c");
        assert_eq!(moved.content, "same");
        assert_eq!(moved.cause_by, cause_by);
    }

    #[test]
    fn new_generates_a_distinct_identifier_each_time() {
        let first = Message::new("agent-d".to_string(), "same".to_string(), None);
        let second = Message::new("agent-d".to_string(), "same".to_string(), None);

        assert_ne!(first.id, second.id);
        assert_ne!(first, second);
    }
}
