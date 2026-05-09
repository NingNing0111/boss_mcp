use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::agent::types::{ConversationTurn, SCHEMA_VERSION};

/// Maximum number of conversation turns to retain in session history.
/// Older turns are truncated from the beginning when this cap is exceeded.
pub const MAX_HISTORY_TURNS: usize = 50;

/// A session tracking conversation state with a candidate.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CandidateSession {
    /// Schema version for migration support.
    pub schema_version: String,

    /// Unique session identifier (keyed by session_id, not candidate_id).
    pub session_id: String,

    /// Unique candidate identifier (user_id).
    pub candidate_id: String,

    /// Conversation history with this candidate.
    pub conversation_history: Vec<ConversationTurn>,

    /// Timestamp when this session was first created.
    pub created_at: DateTime<Utc>,

    /// Timestamp when this session was last updated.
    pub updated_at: DateTime<Utc>,

    /// Optional: the position URL currently being discussed.
    pub active_position_url: Option<String>,
}

impl CandidateSession {
    /// Create a new empty session for the given session_id and candidate_id.
    pub fn new(session_id: impl Into<String>, candidate_id: impl Into<String>) -> Self {
        let now = Utc::now();
        Self {
            schema_version: SCHEMA_VERSION.to_string(),
            session_id: session_id.into(),
            candidate_id: candidate_id.into(),
            conversation_history: Vec::new(),
            created_at: now,
            updated_at: now,
            active_position_url: None,
        }
    }

    /// Deterministic file name for persisting this session.
    /// Keyed by session_id to ensure sessions are distinct per session.
    pub fn file_name(&self) -> String {
        session_file_name(&self.session_id)
    }

    /// Add a turn to the conversation history. Returns a new session with the
    /// updated history; does not mutate self. History is bounded to
    /// MAX_HISTORY_TURNS -- oldest turns are dropped when the cap is exceeded.
    pub fn with_turn(&self, turn: ConversationTurn) -> Self {
        let mut updated = self.clone();
        updated.conversation_history.push(turn);
        // Enforce history cap: keep only the most recent MAX_HISTORY_TURNS turns.
        if updated.conversation_history.len() > MAX_HISTORY_TURNS {
            let excess = updated.conversation_history.len() - MAX_HISTORY_TURNS;
            updated.conversation_history.drain(0..excess);
        }
        updated.updated_at = Utc::now();
        updated
    }

    /// Set the active position URL. Returns a new session; does not mutate self.
    pub fn with_position(&self, position_url: impl Into<String>) -> Self {
        let mut updated = self.clone();
        updated.active_position_url = Some(position_url.into());
        updated.updated_at = Utc::now();
        updated
    }
}

pub(crate) fn session_file_name(session_id: &str) -> String {
    format!("{}.json", sanitize_filename(session_id))
}

fn sanitize_filename(input: &str) -> String {
    input
        .replace(['/', '\\', ':', '*', '?', '"', '<', '>', '|'], "_")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::agent::types::ConversationRole;

    #[test]
    fn new_session_has_empty_history() {
        let session = CandidateSession::new("sess-1", "user-1");
        assert_eq!(session.session_id, "sess-1");
        assert_eq!(session.candidate_id, "user-1");
        assert!(session.conversation_history.is_empty());
        assert_eq!(session.schema_version, SCHEMA_VERSION);
    }

    #[test]
    fn with_turn_appends_to_history() {
        let session = CandidateSession::new("sess-1", "user-1");
        let turn = ConversationTurn {
            role: ConversationRole::Candidate,
            content: "Hello".to_string(),
        };
        let updated = session.with_turn(turn);
        assert_eq!(updated.conversation_history.len(), 1);
        assert_eq!(updated.conversation_history[0].content, "Hello");
        // Original is unchanged
        assert!(session.conversation_history.is_empty());
    }

    #[test]
    fn with_position_sets_active_url() {
        let session = CandidateSession::new("sess-1", "user-1");
        let updated = session.with_position("https://example.com/job/1");
        assert_eq!(
            updated.active_position_url,
            Some("https://example.com/job/1".to_string())
        );
        assert!(session.active_position_url.is_none());
    }

    #[test]
    fn file_name_sanitizes_special_chars() {
        let session = CandidateSession::new("sess/123:456", "user-1");
        assert_eq!(session.file_name(), "sess_123_456.json");
    }

    #[test]
    fn timestamps_set_on_creation() {
        let session = CandidateSession::new("sess-1", "user-1");
        assert!(session.created_at <= Utc::now());
        assert_eq!(session.created_at, session.updated_at);
    }

    #[test]
    fn history_is_capped_at_max_turns() {
        let mut session = CandidateSession::new("sess-1", "user-1");
        // Add MAX_HISTORY_TURNS + 10 turns
        for i in 0..(MAX_HISTORY_TURNS + 10) {
            session = session.with_turn(ConversationTurn {
                role: ConversationRole::Candidate,
                content: format!("message-{i}"),
            });
        }
        assert_eq!(
            session.conversation_history.len(),
            MAX_HISTORY_TURNS,
            "history should be capped at MAX_HISTORY_TURNS"
        );
        // The oldest turns should be dropped; we should keep the most recent ones.
        let first_kept = &session.conversation_history[0].content;
        assert!(
            first_kept.contains("message-10"),
            "oldest kept message should be message-10, got: {first_kept}"
        );
    }

    #[test]
    fn history_stays_within_cap_for_small_counts() {
        let mut session = CandidateSession::new("sess-1", "user-1");
        for i in 0..5 {
            session = session.with_turn(ConversationTurn {
                role: ConversationRole::Candidate,
                content: format!("msg-{i}"),
            });
        }
        assert_eq!(session.conversation_history.len(), 5);
    }
}
