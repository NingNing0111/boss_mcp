use async_trait::async_trait;

use crate::agent::error::AgentError;
use crate::agent::types::ResponderInput;

/// Trait for generating a response to a candidate message.
#[async_trait]
pub trait Responder: Send + Sync {
    /// Generate a reply message based on the responder input.
    async fn respond(&self, input: &ResponderInput) -> std::result::Result<String, AgentError>;
}

/// Rig-based responder that uses the rig completion pipeline.
pub struct RigResponder {
    api_key: String,
    base_url: String,
    model_name: String,
}

impl RigResponder {
    pub fn new(api_key: impl Into<String>, base_url: impl Into<String>, model_name: impl Into<String>) -> Self {
        Self {
            api_key: api_key.into(),
            base_url: base_url.into(),
            model_name: model_name.into(),
        }
    }
}

#[async_trait]
impl Responder for RigResponder {
    async fn respond(&self, input: &ResponderInput) -> std::result::Result<String, AgentError> {
        use rig::completion::Prompt;
        use rig::providers::openai;

        let client = openai::Client::from_url(&self.api_key, &self.base_url);
        let model = client.completion_model(&self.model_name);

        let agent = rig::agent::AgentBuilder::new(model)
            .preamble(&build_responder_preamble(input))
            .build();

        let prompt = build_responder_prompt(input);
        let response = agent
            .prompt(prompt)
            .await
            .map_err(|e| AgentError::ResponseGeneration(e.to_string()))?;

        Ok(response)
    }
}

fn build_responder_preamble(input: &ResponderInput) -> String {
    let company = input.company_name.as_deref().unwrap_or("our company");
    format!(
        "You are a professional recruitment assistant for {company}. \
         Your role is to engage with candidates in a friendly, professional manner. \
         Be concise and helpful. Respond in the same language as the candidate's messages."
    )
}

fn build_responder_prompt(input: &ResponderInput) -> String {
    let mut parts = Vec::new();

    parts.push(format!("Candidate profile: {}", input.candidate_profile_summary));
    parts.push(format!("Position: {}", input.position_url));

    if let Some(company) = &input.company_name {
        parts.push(format!("Company: {company}"));
    }

    if !input.conversation_history.is_empty() {
        parts.push("Conversation history:".to_string());
        for turn in &input.conversation_history {
            let role = match turn.role {
                crate::agent::types::ConversationRole::Candidate => "Candidate",
                crate::agent::types::ConversationRole::Assistant => "Assistant",
            };
            parts.push(format!("  [{role}]: {}", turn.content));
        }
    }

    parts.join("\n")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::agent::types::{ConversationRole, ConversationTurn};

    /// A deterministic fake responder for testing.
    pub struct FakeResponder {
        pub result: std::result::Result<String, AgentError>,
    }

    impl FakeResponder {
        pub fn returning(reply: impl Into<String>) -> Self {
            Self {
                result: Ok(reply.into()),
            }
        }

        pub fn failing(error: AgentError) -> Self {
            Self {
                result: Err(error),
            }
        }
    }

    #[async_trait]
    impl Responder for FakeResponder {
        async fn respond(&self, _input: &ResponderInput) -> std::result::Result<String, AgentError> {
            self.result.clone()
        }
    }

    #[tokio::test]
    async fn fake_responder_returns_preset_reply() {
        let responder = FakeResponder::returning("Welcome!");
        let input = ResponderInput {
            candidate_id: "user-1".to_string(),
            position_url: "https://example.com".to_string(),
            company_name: None,
            candidate_profile_summary: "Test profile".to_string(),
            conversation_history: Vec::new(),
        };
        let result = responder.respond(&input).await.expect("should succeed");
        assert_eq!(result, "Welcome!");
    }

    #[tokio::test]
    async fn fake_responder_returns_preset_error() {
        let responder = FakeResponder::failing(AgentError::ResponseGeneration("fail".to_string()));
        let input = ResponderInput {
            candidate_id: "user-1".to_string(),
            position_url: "https://example.com".to_string(),
            company_name: None,
            candidate_profile_summary: "Test".to_string(),
            conversation_history: Vec::new(),
        };
        let result = responder.respond(&input).await;
        assert!(result.is_err());
    }

    #[test]
    fn build_responder_preamble_includes_company_name() {
        let input = ResponderInput {
            candidate_id: "user-1".to_string(),
            position_url: "url".to_string(),
            company_name: Some("Acme Corp".to_string()),
            candidate_profile_summary: "summary".to_string(),
            conversation_history: Vec::new(),
        };
        let preamble = build_responder_preamble(&input);
        assert!(preamble.contains("Acme Corp"));
    }

    #[test]
    fn build_responder_prompt_includes_conversation_history() {
        let input = ResponderInput {
            candidate_id: "user-1".to_string(),
            position_url: "url".to_string(),
            company_name: None,
            candidate_profile_summary: "A developer".to_string(),
            conversation_history: vec![
                ConversationTurn {
                    role: ConversationRole::Candidate,
                    content: "Hi there".to_string(),
                },
                ConversationTurn {
                    role: ConversationRole::Assistant,
                    content: "Welcome!".to_string(),
                },
            ],
        };
        let prompt = build_responder_prompt(&input);
        assert!(prompt.contains("Hi there"));
        assert!(prompt.contains("Welcome!"));
        assert!(prompt.contains("[Candidate]"));
        assert!(prompt.contains("[Assistant]"));
    }
}
