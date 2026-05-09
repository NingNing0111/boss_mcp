use std::path::PathBuf;

use crate::agent::error::{AgentError, PersistenceError};
use crate::agent::profile::model::CandidateProfile;

/// Trait for profile persistence operations.
pub trait ProfileRepository: Send + Sync {
    /// Load a profile for the given candidate ID. Returns None if not found.
    fn load(&self, candidate_id: &str) -> std::result::Result<Option<CandidateProfile>, AgentError>;

    /// Save a profile, creating or overwriting as needed.
    fn save(&self, profile: &CandidateProfile) -> std::result::Result<(), AgentError>;
}

/// File-system based profile repository. Persists JSON files under a base directory.
pub struct FileProfileRepository {
    base_dir: PathBuf,
}

impl FileProfileRepository {
    pub fn new(base_dir: impl Into<PathBuf>) -> Self {
        Self {
            base_dir: base_dir.into(),
        }
    }

    fn profile_path(&self, candidate_id: &str) -> PathBuf {
        let temp = CandidateProfile::new(candidate_id);
        self.base_dir.join(temp.file_name())
    }

    fn ensure_dir(&self) -> std::result::Result<(), PersistenceError> {
        if !self.base_dir.exists() {
            std::fs::create_dir_all(&self.base_dir)
                .map_err(|e| PersistenceError::io(&self.base_dir, e))?;
        }
        Ok(())
    }
}

impl ProfileRepository for FileProfileRepository {
    fn load(&self, candidate_id: &str) -> std::result::Result<Option<CandidateProfile>, AgentError> {
        let path = self.profile_path(candidate_id);
        if !path.exists() {
            return Ok(None);
        }

        let content = std::fs::read_to_string(&path).map_err(|e| {
            AgentError::ProfilePersistence(PersistenceError::io(&path, e).to_string())
        })?;
        let profile: CandidateProfile = serde_json::from_str(&content).map_err(|e| {
            AgentError::ProfilePersistence(PersistenceError::json(&path, e).to_string())
        })?;
        Ok(Some(profile))
    }

    fn save(&self, profile: &CandidateProfile) -> std::result::Result<(), AgentError> {
        self.ensure_dir()
            .map_err(|e| AgentError::ProfilePersistence(e.to_string()))?;

        let path = self.base_dir.join(profile.file_name());
        let content = serde_json::to_string_pretty(profile).map_err(|e| {
            AgentError::ProfilePersistence(PersistenceError::json(&path, e).to_string())
        })?;

        std::fs::write(&path, content).map_err(|e| {
            AgentError::ProfilePersistence(PersistenceError::io(&path, e).to_string())
        })?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn setup_repo() -> (TempDir, FileProfileRepository) {
        let dir = TempDir::new().expect("temp dir should be created");
        let repo = FileProfileRepository::new(dir.path().join("profiles"));
        (dir, repo)
    }

    #[test]
    fn load_returns_none_for_nonexistent_profile() {
        let (_dir, repo) = setup_repo();
        let result = repo.load("nonexistent").expect("load should succeed");
        assert!(result.is_none());
    }

    #[test]
    fn save_and_load_roundtrip() {
        let (_dir, repo) = setup_repo();
        let profile = CandidateProfile::new("user-123");

        repo.save(&profile).expect("save should succeed");
        let loaded = repo.load("user-123")
            .expect("load should succeed")
            .expect("profile should exist");

        assert_eq!(loaded.candidate_id, "user-123");
        assert_eq!(loaded.schema_version, "1.0.0");
    }

    #[test]
    fn save_creates_directory_if_missing() {
        let dir = TempDir::new().expect("temp dir");
        let nested = dir.path().join("data").join("agent").join("profiles");
        let repo = FileProfileRepository::new(&nested);

        let profile = CandidateProfile::new("user-1");
        repo.save(&profile).expect("should create dirs and save");
        assert!(nested.exists());
    }

    #[test]
    fn save_overwrites_existing_profile() {
        let (_dir, repo) = setup_repo();

        let mut profile = CandidateProfile::new("user-1");
        profile.candidate_name = Some("First".to_string());
        repo.save(&profile).expect("first save");

        profile.candidate_name = Some("Second".to_string());
        repo.save(&profile).expect("second save");

        let loaded = repo.load("user-1").unwrap().unwrap();
        assert_eq!(loaded.candidate_name, Some("Second".to_string()));
    }

    #[test]
    fn persisted_file_is_valid_json_with_pretty_format() {
        let (_dir, repo) = setup_repo();
        let profile = CandidateProfile::new("user-json");
        repo.save(&profile).expect("save");

        let path = repo.profile_path("user-json");
        let content = std::fs::read_to_string(&path).expect("read file");
        // Pretty-printed JSON should have newlines
        assert!(content.contains('\n'), "should be pretty-printed");
        // Should parse as valid JSON
        let parsed: serde_json::Value = serde_json::from_str(&content).expect("valid JSON");
        assert_eq!(parsed["candidate_id"], "user-json");
    }

    #[test]
    fn load_invalid_json_returns_profile_persistence_error() {
        let (_dir, repo) = setup_repo();
        let path = repo.profile_path("broken-user");
        repo.ensure_dir().expect("should create dir");
        std::fs::write(&path, "not json").expect("write invalid json");

        let err = repo.load("broken-user").expect_err("should fail");
        assert!(matches!(err, AgentError::ProfilePersistence(_)));
    }
}
