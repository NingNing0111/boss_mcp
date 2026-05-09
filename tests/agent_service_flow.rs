/// Integration test: end-to-end service flow with fake extractor + fake responder
/// + real temp-dir repositories.
use std::sync::Arc;

use boss_mcp::agent::error::AgentError;
use boss_mcp::agent::llm::extractor::Extractor;
use boss_mcp::agent::llm::responder::Responder;
use boss_mcp::agent::profile::repository::FileProfileRepository;
use boss_mcp::agent::service::RecruitmentAgentService;
use boss_mcp::agent::session::repository::FileSessionRepository;
use boss_mcp::agent::types::{ExtractionContext, ExtractedInfo, RecruitmentAgentRequest, ResponderInput};

use async_trait::async_trait;
use tempfile::TempDir;

// -- Fakes --

struct FakeExtractor {
    result: Arc<Result<ExtractedInfo, AgentError>>,
}

impl FakeExtractor {
    fn returning(info: ExtractedInfo) -> Self {
        Self {
            result: Arc::new(Ok(info)),
        }
    }

    fn failing(error: AgentError) -> Self {
        Self {
            result: Arc::new(Err(error)),
        }
    }
}

#[async_trait]
impl Extractor for FakeExtractor {
    async fn extract(&self, _context: &ExtractionContext) -> Result<ExtractedInfo, AgentError> {
        match self.result.as_ref() {
            Ok(info) => Ok(info.clone()),
            Err(e) => Err(e.clone()),
        }
    }
}

struct FakeResponder {
    result: Arc<Result<String, AgentError>>,
}

impl FakeResponder {
    fn returning(reply: impl Into<String>) -> Self {
        Self {
            result: Arc::new(Ok(reply.into())),
        }
    }

    fn failing(error: AgentError) -> Self {
        Self {
            result: Arc::new(Err(error)),
        }
    }
}

#[async_trait]
impl Responder for FakeResponder {
    async fn respond(&self, _input: &ResponderInput) -> Result<String, AgentError> {
        match self.result.as_ref() {
            Ok(s) => Ok(s.clone()),
            Err(e) => Err(e.clone()),
        }
    }
}

// -- Helpers --

fn setup_service(
    extractor: Box<dyn Extractor>,
    responder: Box<dyn Responder>,
) -> (TempDir, RecruitmentAgentService) {
    let dir = TempDir::new().expect("temp dir");
    let profile_repo = Box::new(FileProfileRepository::new(dir.path().join("profiles")));
    let session_repo = Box::new(FileSessionRepository::new(dir.path().join("sessions")));
    let service = RecruitmentAgentService::new(profile_repo, session_repo, extractor, responder);
    (dir, service)
}

fn simple_request(user_id: &str, session_id: &str, message: &str) -> RecruitmentAgentRequest {
    RecruitmentAgentRequest {
        user_id: user_id.to_string(),
        session_id: session_id.to_string(),
        user_message: message.to_string(),
    }
}

// -- Tests --

#[tokio::test]
async fn first_turn_creates_default_profile_and_returns_reply() {
    let info = ExtractedInfo {
        candidate_name: Some("Alice".to_string()),
        ..Default::default()
    };
    let (_dir, service) = setup_service(
        Box::new(FakeExtractor::returning(info)),
        Box::new(FakeResponder::returning("Welcome, Alice!")),
    );

    let response = service
        .handle_turn(simple_request("user-1", "session-1", "Hi, I'm Alice"))
        .await
        .expect("should succeed");

    assert_eq!(response.reply_text, "Welcome, Alice!");
    assert_eq!(response.profile.candidate_id, "user-1");
    assert_eq!(response.profile.candidate_name, Some("Alice".to_string()));
}

#[tokio::test]
async fn second_turn_refines_profile_incrementally() {
    let info1 = ExtractedInfo {
        candidate_name: Some("Alice".to_string()),
        ..Default::default()
    };
    let info2 = ExtractedInfo {
        skills: vec!["Rust".to_string(), "Go".to_string()],
        ..Default::default()
    };

    // We need a service where the extractor returns different results on
    // successive calls. Use a shared counter approach.
    let call_count = Arc::new(std::sync::Mutex::new(0u32));
    let info2_clone = info2.clone();

    struct SequentialExtractor {
        first: ExtractedInfo,
        second: ExtractedInfo,
        call_count: Arc<std::sync::Mutex<u32>>,
    }

    #[async_trait]
    impl Extractor for SequentialExtractor {
        async fn extract(&self, _context: &ExtractionContext) -> Result<ExtractedInfo, AgentError> {
            let mut count = self.call_count.lock().unwrap();
            *count += 1;
            if *count == 1 {
                Ok(self.first.clone())
            } else {
                Ok(self.second.clone())
            }
        }
    }

    let (_dir, service) = setup_service(
        Box::new(SequentialExtractor {
            first: info1,
            second: info2_clone,
            call_count: call_count.clone(),
        }),
        Box::new(FakeResponder::returning("Got it!")),
    );

    // First turn
    let resp1 = service
        .handle_turn(simple_request("user-2", "session-2", "Hi, I'm Alice"))
        .await
        .expect("first turn");
    assert_eq!(resp1.profile.candidate_name, Some("Alice".to_string()));
    assert!(resp1.profile.candidate_skills.is_empty());

    // Second turn -- profile should accumulate
    let resp2 = service
        .handle_turn(simple_request("user-2", "session-2", "I know Rust and Go"))
        .await
        .expect("second turn");
    assert_eq!(resp2.profile.candidate_name, Some("Alice".to_string()));
    assert_eq!(
        resp2.profile.candidate_skills,
        vec!["Rust".to_string(), "Go".to_string()]
    );
}

#[tokio::test]
async fn validation_error_for_empty_user_message() {
    let (_dir, service) = setup_service(
        Box::new(FakeExtractor::returning(ExtractedInfo::default())),
        Box::new(FakeResponder::returning("ok")),
    );

    let result = service
        .handle_turn(simple_request("user-1", "session-1", "   "))
        .await;
    assert!(result.is_err());
    let err = result.expect_err("should fail");
    assert!(matches!(err, AgentError::Validation(_)));
}

#[tokio::test]
async fn validation_error_for_empty_user_id() {
    let (_dir, service) = setup_service(
        Box::new(FakeExtractor::returning(ExtractedInfo::default())),
        Box::new(FakeResponder::returning("ok")),
    );

    let result = service
        .handle_turn(simple_request("", "session-1", "hello"))
        .await;
    assert!(result.is_err());
    let err = result.expect_err("should fail");
    assert!(matches!(err, AgentError::Validation(_)));
}

#[tokio::test]
async fn response_includes_observations_and_pending_confirmations_from_extractor() {
    let info = ExtractedInfo {
        candidate_name: Some("Bob".to_string()),
        observations: vec![
            "Bob mentioned 5 years of Rust experience".to_string(),
            "Bob is currently employed at TechCorp".to_string(),
        ],
        pending_confirmations: vec![
            "Can you confirm your expected salary range?".to_string(),
        ],
        ..Default::default()
    };
    let (_dir, service) = setup_service(
        Box::new(FakeExtractor::returning(info)),
        Box::new(FakeResponder::returning("Noted.")),
    );

    let response = service
        .handle_turn(simple_request("user-3", "session-3", "I'm Bob with 5 years experience"))
        .await
        .expect("should succeed");

    assert_eq!(response.reply_text, "Noted.");
    // Observations from the extractor must flow through to the response.
    assert_eq!(
        response.observations,
        vec![
            "Bob mentioned 5 years of Rust experience".to_string(),
            "Bob is currently employed at TechCorp".to_string(),
        ]
    );
    // Pending confirmations from the extractor must flow through.
    assert_eq!(
        response.pending_confirmations,
        vec!["Can you confirm your expected salary range?".to_string()]
    );
}

#[tokio::test]
async fn empty_observations_and_confirmations_when_extractor_produces_none() {
    let info = ExtractedInfo {
        candidate_name: Some("Carol".to_string()),
        ..Default::default()
    };
    let (_dir, service) = setup_service(
        Box::new(FakeExtractor::returning(info)),
        Box::new(FakeResponder::returning("Hello!")),
    );

    let response = service
        .handle_turn(simple_request("user-4", "session-4", "Hi"))
        .await
        .expect("should succeed");

    assert!(response.observations.is_empty());
    assert!(response.pending_confirmations.is_empty());
}

// -- Finding 1: session_id-based session isolation --

#[tokio::test]
async fn different_session_ids_for_same_user_load_independent_sessions() {
    let info_a = ExtractedInfo {
        candidate_name: Some("Dave".to_string()),
        ..Default::default()
    };
    let info_b = ExtractedInfo {
        candidate_name: Some("David".to_string()),
        ..Default::default()
    };

    let call_count = Arc::new(std::sync::Mutex::new(0u32));

    struct SwitchExtractor {
        first: ExtractedInfo,
        second: ExtractedInfo,
        call_count: Arc<std::sync::Mutex<u32>>,
    }

    #[async_trait]
    impl Extractor for SwitchExtractor {
        async fn extract(&self, _context: &ExtractionContext) -> Result<ExtractedInfo, AgentError> {
            let mut count = self.call_count.lock().unwrap();
            *count += 1;
            if *count == 1 {
                Ok(self.first.clone())
            } else {
                Ok(self.second.clone())
            }
        }
    }

    let (_dir, service) = setup_service(
        Box::new(SwitchExtractor {
            first: info_a,
            second: info_b,
            call_count: call_count.clone(),
        }),
        Box::new(FakeResponder::returning("OK")),
    );

    // Same user, different sessions
    let resp_a = service
        .handle_turn(simple_request("user-5", "session-alpha", "I'm Dave"))
        .await
        .expect("session A should succeed");
    assert_eq!(resp_a.profile.candidate_name, Some("Dave".to_string()));

    let resp_b = service
        .handle_turn(simple_request("user-5", "session-beta", "Call me David"))
        .await
        .expect("session B should succeed");
    assert_eq!(resp_b.profile.candidate_name, Some("David".to_string()));
}
