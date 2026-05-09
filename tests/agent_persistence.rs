/// Integration test: persistence and failure-safety scenarios.
///
/// Tests that:
/// - State survives across multiple turns (persistence)
/// - Extraction failure leaves persisted state unchanged
/// - Response failure leaves persisted state unchanged
/// - Observations and pending_confirmations are populated from extractor output
/// - Sessions are isolated by session_id across service restarts
/// - Conversation history is bounded at MAX_HISTORY_TURNS
use std::sync::Arc;

use boss_mcp::agent::error::AgentError;
use boss_mcp::agent::llm::extractor::Extractor;
use boss_mcp::agent::llm::responder::Responder;
use boss_mcp::agent::profile::repository::{FileProfileRepository, ProfileRepository};
use boss_mcp::agent::service::RecruitmentAgentService;
use boss_mcp::agent::session::model::MAX_HISTORY_TURNS;
use boss_mcp::agent::session::repository::{FileSessionRepository, SessionRepository};
use boss_mcp::agent::types::{ConversationRole, ExtractedInfo, ExtractionContext, RecruitmentAgentRequest, ResponderInput};

use async_trait::async_trait;
use tempfile::TempDir;

// -- Fakes (duplicated from agent_service_flow to keep integration tests independent) --

struct FakeExtractor {
    result: Arc<Result<ExtractedInfo, AgentError>>,
}

impl FakeExtractor {
    fn returning(info: ExtractedInfo) -> Self {
        Self { result: Arc::new(Ok(info)) }
    }
    fn failing(error: AgentError) -> Self {
        Self { result: Arc::new(Err(error)) }
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
        Self { result: Arc::new(Ok(reply.into())) }
    }
    fn failing(error: AgentError) -> Self {
        Self { result: Arc::new(Err(error)) }
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

fn request(user_id: &str, message: &str) -> RecruitmentAgentRequest {
    RecruitmentAgentRequest {
        user_id: user_id.to_string(),
        session_id: "session-1".to_string(),
        user_message: message.to_string(),
    }
}

// -- Tests --

#[tokio::test]
async fn persistence_across_multiple_turns() {
    let info1 = ExtractedInfo {
        candidate_name: Some("Carol".to_string()),
        ..Default::default()
    };
    let info2 = ExtractedInfo {
        location: Some("Shanghai".to_string()),
        ..Default::default()
    };

    // Use sequential extractor
    let call_count = Arc::new(std::sync::Mutex::new(0u32));

    struct SeqExtractor {
        infos: Vec<ExtractedInfo>,
        call_count: Arc<std::sync::Mutex<u32>>,
    }

    #[async_trait]
    impl Extractor for SeqExtractor {
        async fn extract(&self, _context: &ExtractionContext) -> Result<ExtractedInfo, AgentError> {
            let mut count = self.call_count.lock().unwrap();
            let idx = (*count as usize).min(self.infos.len().saturating_sub(1));
            *count += 1;
            Ok(self.infos[idx].clone())
        }
    }

    let (_dir, service) = setup_service(
        Box::new(SeqExtractor {
            infos: vec![info1, info2],
            call_count: call_count.clone(),
        }),
        Box::new(FakeResponder::returning("ok")),
    );

    // Turn 1
    let resp1 = service
        .handle_turn(request("user-p1", "I'm Carol"))
        .await
        .expect("turn 1");
    assert_eq!(resp1.profile.candidate_name, Some("Carol".to_string()));
    assert_eq!(resp1.profile.location, None);

    // Turn 2 -- profile should have both name and location
    let resp2 = service
        .handle_turn(request("user-p1", "I live in Shanghai"))
        .await
        .expect("turn 2");
    assert_eq!(resp2.profile.candidate_name, Some("Carol".to_string()));
    assert_eq!(resp2.profile.location, Some("Shanghai".to_string()));
}

#[tokio::test]
async fn extraction_failure_leaves_persisted_state_unchanged() {
    // First, establish state with a successful turn
    let info = ExtractedInfo {
        candidate_name: Some("Dave".to_string()),
        ..Default::default()
    };

    let (dir, service) = setup_service(
        Box::new(FakeExtractor::returning(info)),
        Box::new(FakeResponder::returning("Welcome!")),
    );

    let resp1 = service
        .handle_turn(request("user-fail1", "I'm Dave"))
        .await
        .expect("first turn should succeed");
    assert_eq!(resp1.profile.candidate_name, Some("Dave".to_string()));

    // Now create a new service with a failing extractor, but same temp dir
    let profile_repo = Box::new(FileProfileRepository::new(dir.path().join("profiles")));
    let session_repo = Box::new(FileSessionRepository::new(dir.path().join("sessions")));
    let failing_service = RecruitmentAgentService::new(
        profile_repo,
        session_repo,
        Box::new(FakeExtractor::failing(AgentError::Extraction(
            "LLM down".to_string(),
        ))),
        Box::new(FakeResponder::returning("should not reach")),
    );

    let result = failing_service
        .handle_turn(request("user-fail1", "More info"))
        .await;
    assert!(result.is_err());

    // Verify that persisted profile still has the original data
    let check_repo = FileProfileRepository::new(dir.path().join("profiles"));
    let profile = check_repo
        .load("user-fail1")
        .expect("load should succeed")
        .expect("profile should exist");
    assert_eq!(profile.candidate_name, Some("Dave".to_string()));
}

#[tokio::test]
async fn response_failure_leaves_persisted_state_unchanged() {
    let info = ExtractedInfo {
        candidate_name: Some("Eve".to_string()),
        ..Default::default()
    };

    let (dir, service) = setup_service(
        Box::new(FakeExtractor::returning(info)),
        Box::new(FakeResponder::returning("Hello!")),
    );

    let resp1 = service
        .handle_turn(request("user-fail2", "I'm Eve"))
        .await
        .expect("first turn");
    assert_eq!(resp1.profile.candidate_name, Some("Eve".to_string()));

    // New service with failing responder, same temp dir
    let info2 = ExtractedInfo {
        location: Some("Beijing".to_string()),
        ..Default::default()
    };
    let profile_repo = Box::new(FileProfileRepository::new(dir.path().join("profiles")));
    let session_repo = Box::new(FileSessionRepository::new(dir.path().join("sessions")));
    let failing_service = RecruitmentAgentService::new(
        profile_repo,
        session_repo,
        Box::new(FakeExtractor::returning(info2)),
        Box::new(FakeResponder::failing(AgentError::ResponseGeneration(
            "timeout".to_string(),
        ))),
    );

    let result = failing_service
        .handle_turn(request("user-fail2", "I'm in Beijing"))
        .await;
    assert!(result.is_err());

    // Profile should still have Eve, NOT updated with Beijing
    let check_repo = FileProfileRepository::new(dir.path().join("profiles"));
    let profile = check_repo
        .load("user-fail2")
        .expect("load should succeed")
        .expect("profile should exist");
    assert_eq!(profile.candidate_name, Some("Eve".to_string()));
    assert_eq!(
        profile.location, None,
        "location should not have been persisted since responder failed"
    );
}

#[tokio::test]
async fn observations_and_pending_confirmations_flow_through_persistence() {
    let info = ExtractedInfo {
        candidate_name: Some("Frank".to_string()),
        observations: vec![
            "Frank has 8 years of Go experience".to_string(),
            "Frank prefers remote work".to_string(),
        ],
        pending_confirmations: vec![
            "Please confirm your expected salary range".to_string(),
            "Are you open to relocation?".to_string(),
        ],
        ..Default::default()
    };

    let (dir, service) = setup_service(
        Box::new(FakeExtractor::returning(info)),
        Box::new(FakeResponder::returning("Noted, Frank.")),
    );

    let response = service
        .handle_turn(RecruitmentAgentRequest {
            user_id: "user-obs1".to_string(),
            session_id: "session-obs1".to_string(),
            user_message: "I have 8 years of Go and prefer remote".to_string(),
        })
        .await
        .expect("should succeed");

    assert_eq!(response.reply_text, "Noted, Frank.");
    assert_eq!(
        response.observations,
        vec![
            "Frank has 8 years of Go experience".to_string(),
            "Frank prefers remote work".to_string(),
        ],
        "observations from extractor must appear in response"
    );
    assert_eq!(
        response.pending_confirmations,
        vec![
            "Please confirm your expected salary range".to_string(),
            "Are you open to relocation?".to_string(),
        ],
        "pending_confirmations from extractor must appear in response"
    );

    // Verify that a second turn with no observations/confirmations returns empty
    let info2 = ExtractedInfo {
        candidate_name: Some("Frank".to_string()),
        ..Default::default()
    };

    let service2 = RecruitmentAgentService::new(
        Box::new(FileProfileRepository::new(dir.path().join("profiles"))),
        Box::new(FileSessionRepository::new(dir.path().join("sessions"))),
        Box::new(FakeExtractor::returning(info2)),
        Box::new(FakeResponder::returning("Anything else?")),
    );

    let response2 = service2
        .handle_turn(RecruitmentAgentRequest {
            user_id: "user-obs1".to_string(),
            session_id: "session-obs1".to_string(),
            user_message: "No that's all for now".to_string(),
        })
        .await
        .expect("should succeed");

    assert!(
        response2.observations.is_empty(),
        "second turn with no observations should return empty observations"
    );
    assert!(
        response2.pending_confirmations.is_empty(),
        "second turn with no confirmations should return empty pending_confirmations"
    );
}

#[tokio::test]
async fn session_isolation_by_session_id_across_service_restarts() {
    // Same user creates two different sessions. After a service restart (simulated
    // by creating a new service pointing at the same temp dir), each session must
    // load its own independent conversation history keyed by session_id.

    let call_count = Arc::new(std::sync::Mutex::new(0u32));

    struct SeqExtractor {
        infos: Vec<ExtractedInfo>,
        call_count: Arc<std::sync::Mutex<u32>>,
    }

    #[async_trait]
    impl Extractor for SeqExtractor {
        async fn extract(&self, _context: &ExtractionContext) -> Result<ExtractedInfo, AgentError> {
            let mut count = self.call_count.lock().unwrap();
            let idx = (*count as usize).min(self.infos.len().saturating_sub(1));
            *count += 1;
            Ok(self.infos[idx].clone())
        }
    }

    let info_a = ExtractedInfo {
        candidate_name: Some("Grace".to_string()),
        ..Default::default()
    };
    let info_b = ExtractedInfo {
        candidate_name: Some("Greta".to_string()),
        ..Default::default()
    };

    let (dir, service) = setup_service(
        Box::new(SeqExtractor {
            infos: vec![info_a, info_b],
            call_count: call_count.clone(),
        }),
        Box::new(FakeResponder::returning("OK")),
    );

    // Turn in session-alpha for user-x
    let resp_a = service
        .handle_turn(RecruitmentAgentRequest {
            user_id: "user-x".to_string(),
            session_id: "session-alpha".to_string(),
            user_message: "I'm Grace".to_string(),
        })
        .await
        .expect("session A should succeed");
    assert_eq!(resp_a.profile.candidate_name, Some("Grace".to_string()));

    // Turn in session-beta for the same user-x
    let resp_b = service
        .handle_turn(RecruitmentAgentRequest {
            user_id: "user-x".to_string(),
            session_id: "session-beta".to_string(),
            user_message: "Call me Greta".to_string(),
        })
        .await
        .expect("session B should succeed");
    assert_eq!(resp_b.profile.candidate_name, Some("Greta".to_string()));

    // Verify through direct repo access that sessions are distinct
    let session_repo = FileSessionRepository::new(dir.path().join("sessions"));
    let loaded_alpha = session_repo.load("session-alpha").unwrap().unwrap();
    let loaded_beta = session_repo.load("session-beta").unwrap().unwrap();

    assert_eq!(loaded_alpha.session_id, "session-alpha");
    assert_eq!(loaded_beta.session_id, "session-beta");
    // Both sessions belong to the same user
    assert_eq!(loaded_alpha.candidate_id, "user-x");
    assert_eq!(loaded_beta.candidate_id, "user-x");
    // Each session has its own independent history
    assert_eq!(loaded_alpha.conversation_history.len(), 2); // user msg + assistant reply
    assert_eq!(loaded_beta.conversation_history.len(), 2);
    assert_eq!(loaded_alpha.conversation_history[0].content, "I'm Grace");
    assert_eq!(loaded_beta.conversation_history[0].content, "Call me Greta");
}

#[tokio::test]
async fn conversation_history_is_bounded_at_max_turns() {
    // Drive enough turns through the service to exceed MAX_HISTORY_TURNS,
    // then verify the persisted session is capped.

    let info = ExtractedInfo {
        candidate_name: Some("Heidi".to_string()),
        ..Default::default()
    };

    let (dir, service) = setup_service(
        Box::new(FakeExtractor::returning(info)),
        Box::new(FakeResponder::returning("ack")),
    );

    // Each handle_turn adds 2 turns (candidate + assistant).
    // We need enough turns to exceed MAX_HISTORY_TURNS.
    let turns_needed = (MAX_HISTORY_TURNS / 2) + 10;

    for i in 0..turns_needed {
        service
            .handle_turn(RecruitmentAgentRequest {
                user_id: "user-cap1".to_string(),
                session_id: "session-cap1".to_string(),
                user_message: format!("message-{i}"),
            })
            .await
            .expect("turn should succeed");
    }

    // Verify persisted session is capped
    let session_repo = FileSessionRepository::new(dir.path().join("sessions"));
    let session = session_repo
        .load("session-cap1")
        .expect("load should succeed")
        .expect("session should exist");

    assert_eq!(
        session.conversation_history.len(),
        MAX_HISTORY_TURNS,
        "persisted session history must be capped at MAX_HISTORY_TURNS"
    );

    // Verify the oldest kept messages are the most recent ones (not the first)
    let first_kept = &session.conversation_history[0].content;
    assert!(
        !first_kept.contains("message-0"),
        "oldest messages should have been evicted, but found: {first_kept}"
    );
}

#[tokio::test]
async fn extractor_receives_context_with_profile_and_history() {
    // Verify that the extractor actually receives an ExtractionContext with
    // profile_summary and conversation_history populated, not just the raw text.

    let captured_context: Arc<std::sync::Mutex<Option<ExtractionContext>>> =
        Arc::new(std::sync::Mutex::new(None));
    let captured_clone = captured_context.clone();

    struct CapturingExtractor {
        captured: Arc<std::sync::Mutex<Option<ExtractionContext>>>,
    }

    #[async_trait]
    impl Extractor for CapturingExtractor {
        async fn extract(&self, context: &ExtractionContext) -> Result<ExtractedInfo, AgentError> {
            let mut guard = self.captured.lock().unwrap();
            *guard = Some(context.clone());
            Ok(ExtractedInfo {
                candidate_name: Some("Ivan".to_string()),
                ..Default::default()
            })
        }
    }

    let (_dir, service) = setup_service(
        Box::new(CapturingExtractor {
            captured: captured_clone,
        }),
        Box::new(FakeResponder::returning("Hello")),
    );

    // First turn -- profile_summary should be empty since profile is new
    service
        .handle_turn(RecruitmentAgentRequest {
            user_id: "user-ctx1".to_string(),
            session_id: "session-ctx1".to_string(),
            user_message: "I'm Ivan".to_string(),
        })
        .await
        .expect("first turn");

    let ctx1 = captured_context.lock().unwrap().take().unwrap();
    assert_eq!(ctx1.user_message, "I'm Ivan");
    assert!(
        ctx1.profile_summary.contains("No profile information"),
        "first turn profile summary should be empty/default, got: {}",
        ctx1.profile_summary
    );
    assert!(
        ctx1.conversation_history.is_empty(),
        "first turn should have no history"
    );

    // Second turn -- profile_summary should now contain "Ivan", history should have 2 turns
    service
        .handle_turn(RecruitmentAgentRequest {
            user_id: "user-ctx1".to_string(),
            session_id: "session-ctx1".to_string(),
            user_message: "I know Rust".to_string(),
        })
        .await
        .expect("second turn");

    let ctx2 = captured_context.lock().unwrap().take().unwrap();
    assert_eq!(ctx2.user_message, "I know Rust");
    assert!(
        ctx2.profile_summary.contains("Ivan"),
        "second turn profile summary should contain Ivan, got: {}",
        ctx2.profile_summary
    );
    assert_eq!(
        ctx2.conversation_history.len(),
        2,
        "second turn should have 2 history turns (candidate + assistant from first turn)"
    );
    assert_eq!(ctx2.conversation_history[0].role, ConversationRole::Candidate);
    assert_eq!(ctx2.conversation_history[0].content, "I'm Ivan");
    assert_eq!(ctx2.conversation_history[1].role, ConversationRole::Assistant);
    assert_eq!(ctx2.conversation_history[1].content, "Hello");
}
