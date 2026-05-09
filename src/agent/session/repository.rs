use std::path::PathBuf;

use crate::agent::error::{AgentError, PersistenceError};
use crate::agent::session::model::{session_file_name, CandidateSession};

/// Trait for session persistence operations.
pub trait SessionRepository: Send + Sync {
    /// Load a session for the given session ID. Returns None if not found.
    fn load(&self, session_id: &str) -> std::result::Result<Option<CandidateSession>, AgentError>;

    /// Save a session, creating or overwriting as needed.
    fn save(&self, session: &CandidateSession) -> std::result::Result<(), AgentError>;
}

/// File-system based session repository. Persists JSON files under a base directory.
/// Files are keyed by session_id, not candidate_id.
pub struct FileSessionRepository {
    base_dir: PathBuf,
}

impl FileSessionRepository {
    pub fn new(base_dir: impl Into<PathBuf>) -> Self {
        Self {
            base_dir: base_dir.into(),
        }
    }

    fn session_path(&self, session_id: &str) -> PathBuf {
        self.base_dir.join(session_file_name(session_id))
    }

    fn ensure_dir(&self) -> std::result::Result<(), PersistenceError> {
        if !self.base_dir.exists() {
            std::fs::create_dir_all(&self.base_dir)
                .map_err(|e| PersistenceError::io(&self.base_dir, e))?;
        }
        Ok(())
    }
}

impl SessionRepository for FileSessionRepository {
    fn load(&self, session_id: &str) -> std::result::Result<Option<CandidateSession>, AgentError> {
        let path = self.session_path(session_id);
        if !path.exists() {
            return Ok(None);
        }

        let content = std::fs::read_to_string(&path).map_err(|e| {
            AgentError::SessionPersistence(PersistenceError::io(&path, e).to_string())
        })?;
        let session: CandidateSession = serde_json::from_str(&content).map_err(|e| {
            AgentError::SessionPersistence(PersistenceError::json(&path, e).to_string())
        })?;
        Ok(Some(session))
    }

    fn save(&self, session: &CandidateSession) -> std::result::Result<(), AgentError> {
        self.ensure_dir()
            .map_err(|e| AgentError::SessionPersistence(e.to_string()))?;

        let path = self.session_path(&session.session_id);
        let content = serde_json::to_string_pretty(session).map_err(|e| {
            AgentError::SessionPersistence(PersistenceError::json(&path, e).to_string())
        })?;

        std::fs::write(&path, content).map_err(|e| {
            AgentError::SessionPersistence(PersistenceError::io(&path, e).to_string())
        })?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::agent::types::{ConversationRole, ConversationTurn};
    use tempfile::TempDir;

    fn setup_repo() -> (TempDir, FileSessionRepository) {
        let dir = TempDir::new().expect("temp dir should be created");
        let repo = FileSessionRepository::new(dir.path().join("sessions"));
        (dir, repo)
    }

    #[test]
    fn load_returns_none_for_nonexistent_session() {
        let (_dir, repo) = setup_repo();
        let result = repo.load("nonexistent-session").expect("load should succeed");
        assert!(result.is_none());
    }

    #[test]
    fn save_and_load_roundtrip() {
        let (_dir, repo) = setup_repo();
        let session = CandidateSession::new("session-123", "user-123");

        repo.save(&session).expect("save should succeed");
        let loaded = repo.load("session-123")
            .expect("load should succeed")
            .expect("session should exist");

        assert_eq!(loaded.session_id, "session-123");
        assert_eq!(loaded.candidate_id, "user-123");
        assert!(loaded.conversation_history.is_empty());
    }

    #[test]
    fn save_and_load_with_conversation_history() {
        let (_dir, repo) = setup_repo();
        let session = CandidateSession::new("session-456", "user-456")
            .with_turn(ConversationTurn {
                role: ConversationRole::Candidate,
                content: "Hi".to_string(),
            })
            .with_turn(ConversationTurn {
                role: ConversationRole::Assistant,
                content: "Hello!".to_string(),
            });

        repo.save(&session).expect("save should succeed");
        let loaded = repo.load("session-456").unwrap().unwrap();

        assert_eq!(loaded.conversation_history.len(), 2);
        assert_eq!(loaded.conversation_history[0].content, "Hi");
        assert_eq!(loaded.conversation_history[1].content, "Hello!");
    }

    #[test]
    fn save_creates_directory_if_missing() {
        let dir = TempDir::new().expect("temp dir");
        let nested = dir.path().join("data").join("agent").join("sessions");
        let repo = FileSessionRepository::new(&nested);

        let session = CandidateSession::new("session-1", "user-1");
        repo.save(&session).expect("should create dirs and save");
        assert!(nested.exists());
    }

    #[test]
    fn save_overwrites_existing_session() {
        let (_dir, repo) = setup_repo();

        let session = CandidateSession::new("session-1", "user-1");
        repo.save(&session).expect("first save");

        let updated = session.with_turn(ConversationTurn {
            role: ConversationRole::Candidate,
            content: "New message".to_string(),
        });
        repo.save(&updated).expect("second save");

        let loaded = repo.load("session-1").unwrap().unwrap();
        assert_eq!(loaded.conversation_history.len(), 1);
    }

    #[test]
    fn different_session_ids_load_different_sessions() {
        let (_dir, repo) = setup_repo();

        let session_a = CandidateSession::new("session-alpha", "user-1");
        let session_b = CandidateSession::new("session-beta", "user-1");

        repo.save(&session_a).expect("save A");
        repo.save(&session_b).expect("save B");

        let loaded_a = repo.load("session-alpha").unwrap().unwrap();
        let loaded_b = repo.load("session-beta").unwrap().unwrap();

        assert_eq!(loaded_a.session_id, "session-alpha");
        assert_eq!(loaded_b.session_id, "session-beta");
        // Both belong to same user but are distinct sessions
        assert_eq!(loaded_a.candidate_id, "user-1");
        assert_eq!(loaded_b.candidate_id, "user-1");
    }

    #[test]
    fn load_invalid_json_returns_session_persistence_error() {
        let (_dir, repo) = setup_repo();
        let path = repo.session_path("broken-session");
        repo.ensure_dir().expect("should create dir");
        std::fs::write(&path, "not json").expect("write invalid json");

        let err = repo.load("broken-session").expect_err("should fail");
        assert!(matches!(err, AgentError::SessionPersistence(_)));
    }
}
