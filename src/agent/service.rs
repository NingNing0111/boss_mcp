use crate::agent::error::AgentError;
use crate::agent::llm::extractor::Extractor;
use crate::agent::llm::responder::Responder;
use crate::agent::profile::merge::merge_profile;
use crate::agent::profile::model::CandidateProfile;
use crate::agent::profile::repository::ProfileRepository;
use crate::agent::session::model::CandidateSession;
use crate::agent::session::repository::SessionRepository;
use crate::agent::types::{ConversationRole, ConversationTurn, ExtractionContext, RecruitmentAgentRequest, RecruitmentAgentResponse, ResponderInput};

/// Main orchestration service for the recruitment agent.
///
/// Orchestrates the flow: validate -> load state -> extract -> merge ->
/// respond -> persist -> return result.
///
/// Constructed with injectable collaborators so tests can provide fakes.
pub struct RecruitmentAgentService {
    profile_repo: Box<dyn ProfileRepository>,
    session_repo: Box<dyn SessionRepository>,
    extractor: Box<dyn Extractor>,
    responder: Box<dyn Responder>,
}

impl RecruitmentAgentService {
    /// Create a new service with the given collaborators.
    pub fn new(
        profile_repo: Box<dyn ProfileRepository>,
        session_repo: Box<dyn SessionRepository>,
        extractor: Box<dyn Extractor>,
        responder: Box<dyn Responder>,
    ) -> Self {
        Self {
            profile_repo,
            session_repo,
            extractor,
            responder,
        }
    }

    /// Handle a single conversation turn.
    ///
    /// Steps:
    /// 1. Validate the request.
    /// 2. Load existing profile/session or initialize defaults.
    /// 3. Call the extractor with message + profile/session context.
    /// 4. Merge extracted info into the profile deterministically.
    /// 5. Call the responder to generate a reply.
    /// 6. Update the session with the new conversation turns.
    /// 7. Persist profile and session.
    /// 8. Return the assembled response.
    ///
    /// If extraction or response generation fails, the persisted state is
    /// left unchanged -- no partial/corrupt data is written.
    pub async fn handle_turn(
        &self,
        request: RecruitmentAgentRequest,
    ) -> Result<RecruitmentAgentResponse, AgentError> {
        // 1. Validate
        request.validate()?;

        let candidate_id = &request.user_id;
        let session_id = &request.session_id;

        // 2. Load existing state or initialize defaults
        let profile = self
            .profile_repo
            .load(candidate_id)?
            .unwrap_or_else(|| CandidateProfile::new(candidate_id));

        let session = self
            .session_repo
            .load(session_id)?
            .unwrap_or_else(|| CandidateSession::new(session_id, candidate_id));

        // 3. Extract with context -- if this fails, we return early without persisting
        let extraction_context = ExtractionContext {
            user_message: request.user_message.clone(),
            profile_summary: profile.summary(),
            conversation_history: session.conversation_history.clone(),
        };
        let extracted = self.extractor.extract(&extraction_context).await?;

        // 4. Merge profile updates deterministically
        let merged_profile = merge_profile(&profile, &extracted);

        // 5. Build responder input and generate reply
        let responder_input = ResponderInput {
            candidate_id: candidate_id.clone(),
            position_url: session
                .active_position_url
                .clone()
                .unwrap_or_default(),
            company_name: None,
            candidate_profile_summary: merged_profile.summary(),
            conversation_history: session.conversation_history.clone(),
        };

        // If responder fails, we return early without persisting
        let reply_text = self.responder.respond(&responder_input).await?;

        // 6. Update session with new turns
        let updated_session = session
            .with_turn(ConversationTurn {
                role: ConversationRole::Candidate,
                content: request.user_message.clone(),
            })
            .with_turn(ConversationTurn {
                role: ConversationRole::Assistant,
                content: reply_text.clone(),
            });

        // 7. Persist both profile and session
        self.profile_repo.save(&merged_profile)?;
        self.session_repo.save(&updated_session)?;

        // 8. Assemble and return response
        Ok(RecruitmentAgentResponse {
            reply_text,
            profile: merged_profile,
            observations: extracted.observations,
            pending_confirmations: extracted.pending_confirmations,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn service_new_constructs_with_injected_deps() {
        let dir = TempDir::new().expect("temp dir");
        let _service = RecruitmentAgentService::new(
            Box::new(
                crate::agent::profile::repository::FileProfileRepository::new(
                    dir.path().join("profiles"),
                ),
            ),
            Box::new(
                crate::agent::session::repository::FileSessionRepository::new(
                    dir.path().join("sessions"),
                ),
            ),
            Box::new(crate::agent::llm::extractor::RigExtractor::new(
                "key",
                "url",
                "model",
            )),
            Box::new(crate::agent::llm::responder::RigResponder::new(
                "key",
                "url",
                "model",
            )),
        );
    }
}
