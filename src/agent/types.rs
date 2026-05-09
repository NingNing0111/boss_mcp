use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use super::error::AgentError;

/// Current schema version for persisted documents.
pub const SCHEMA_VERSION: &str = "1.0.0";

/// Context provided to the extractor alongside the user message.
/// Includes the user message plus profile/session context for richer extraction.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ExtractionContext {
    /// The user's raw message text.
    pub user_message: String,
    /// Human-readable summary of the current candidate profile.
    pub profile_summary: String,
    /// Recent conversation history (already bounded by session policy).
    pub conversation_history: Vec<ConversationTurn>,
}

/// Extracted candidate information from an LLM or external source.
/// Used as the "candidate facts" input to the profile merge step.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default, JsonSchema)]
pub struct ExtractedInfo {
    pub candidate_name: Option<String>,
    pub skills: Vec<String>,
    pub experience_years: Option<u32>,
    pub education: Option<String>,
    pub current_company: Option<String>,
    pub desired_salary: Option<String>,
    pub location: Option<String>,
    pub position_title: Option<String>,
    /// Observations about the candidate from this turn.
    #[serde(default)]
    pub observations: Vec<String>,
    /// Questions or facts that need user confirmation.
    #[serde(default)]
    pub pending_confirmations: Vec<String>,
}

/// Structured data sent to the responder to generate a reply.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ResponderInput {
    pub candidate_id: String,
    pub position_url: String,
    pub company_name: Option<String>,
    pub candidate_profile_summary: String,
    pub conversation_history: Vec<ConversationTurn>,
}

/// A single turn in the conversation history.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ConversationTurn {
    pub role: ConversationRole,
    pub content: String,
}

/// Role within a conversation turn.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ConversationRole {
    Candidate,
    Assistant,
}

/// Multi-turn request to the recruitment agent service.
/// Used for conversational interactions where the user sends a message
/// and the agent orchestrates extraction, profile merge, and response.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct RecruitmentAgentRequest {
    /// Unique user/candidate identifier.
    pub user_id: String,
    /// Session identifier for this conversation.
    pub session_id: String,
    /// The user's message text for this turn.
    pub user_message: String,
}

impl RecruitmentAgentRequest {
    /// Validate the request fields.
    pub fn validate(&self) -> std::result::Result<(), AgentError> {
        if self.user_id.trim().is_empty() {
            return Err(AgentError::Validation(
                "user_id must not be empty".to_string(),
            ));
        }
        if self.session_id.trim().is_empty() {
            return Err(AgentError::Validation(
                "session_id must not be empty".to_string(),
            ));
        }
        if self.user_message.trim().is_empty() {
            return Err(AgentError::Validation(
                "user_message must not be empty".to_string(),
            ));
        }
        Ok(())
    }
}

/// Rich response from the multi-turn recruitment agent service.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct RecruitmentAgentResponse {
    /// The generated reply text.
    pub reply_text: String,
    /// Current candidate profile after this turn.
    pub profile: crate::agent::profile::CandidateProfile,
    /// Observations made during extraction (if any).
    pub observations: Vec<String>,
    /// Pending confirmations the agent is waiting for from the user.
    pub pending_confirmations: Vec<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extracted_info_default_has_empty_fields() {
        let info = ExtractedInfo::default();
        assert!(info.candidate_name.is_none());
        assert!(info.skills.is_empty());
        assert!(info.experience_years.is_none());
    }

    #[test]
    fn recruitment_request_validates_all_fields() {
        let valid = RecruitmentAgentRequest {
            user_id: "u1".to_string(),
            session_id: "s1".to_string(),
            user_message: "hello".to_string(),
        };
        assert!(valid.validate().is_ok());
    }

    #[test]
    fn recruitment_request_rejects_empty_user_id() {
        let req = RecruitmentAgentRequest {
            user_id: "  ".to_string(),
            session_id: "s1".to_string(),
            user_message: "hello".to_string(),
        };
        let err = req.validate().expect_err("should fail");
        assert!(err.to_string().contains("user_id"));
    }

    #[test]
    fn recruitment_request_rejects_empty_session_id() {
        let req = RecruitmentAgentRequest {
            user_id: "u1".to_string(),
            session_id: "".to_string(),
            user_message: "hello".to_string(),
        };
        let err = req.validate().expect_err("should fail");
        assert!(err.to_string().contains("session_id"));
    }

    #[test]
    fn recruitment_request_rejects_empty_user_message() {
        let req = RecruitmentAgentRequest {
            user_id: "u1".to_string(),
            session_id: "s1".to_string(),
            user_message: "   ".to_string(),
        };
        let err = req.validate().expect_err("should fail");
        assert!(err.to_string().contains("user_message"));
    }
}
