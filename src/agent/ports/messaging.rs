use async_trait::async_trait;

use crate::agent::error::AgentError;

/// Port for sending messages to a candidate via the Boss platform.
#[async_trait]
pub trait MessagingPort: Send + Sync {
    /// Send a message to the given candidate.
    async fn send(&self, candidate_id: &str, message: &str) -> std::result::Result<(), AgentError>;
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{Arc, Mutex};

    pub struct FakeMessagingPort {
        pub sent: Arc<Mutex<Vec<(String, String)>>>,
        pub result: std::result::Result<(), AgentError>,
    }

    impl FakeMessagingPort {
        pub fn new() -> Self {
            Self {
                sent: Arc::new(Mutex::new(Vec::new())),
                result: Ok(()),
            }
        }

        pub fn failing(error: AgentError) -> Self {
            Self {
                sent: Arc::new(Mutex::new(Vec::new())),
                result: Err(error),
            }
        }
    }

    #[async_trait]
    impl MessagingPort for FakeMessagingPort {
        async fn send(&self, candidate_id: &str, message: &str) -> std::result::Result<(), AgentError> {
            self.sent.lock().unwrap().push((candidate_id.to_string(), message.to_string()));
            self.result.clone()
        }
    }

    #[tokio::test]
    async fn fake_messaging_records_sent_messages() {
        let port = FakeMessagingPort::new();
        port.send("user-1", "Hello!").await.expect("should succeed");
        let sent = port.sent.lock().unwrap();
        assert_eq!(sent.len(), 1);
        assert_eq!(sent[0], ("user-1".to_string(), "Hello!".to_string()));
    }
}
