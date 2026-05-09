use crate::agent::types::{ConversationRole, ConversationTurn, ResponderInput};

/// Build a prompt string for the extractor from context.
/// Includes the user message, profile summary, and recent conversation history.
pub fn build_extraction_prompt(
    raw_text: &str,
    profile_summary: &str,
    conversation_history: &[ConversationTurn],
) -> String {
    let mut parts = Vec::new();

    parts.push("Extract the following candidate information from this text.".to_string());

    if !profile_summary.is_empty() {
        parts.push(format!("\nCurrent profile:\n{profile_summary}"));
    }

    if !conversation_history.is_empty() {
        parts.push("\nRecent conversation:".to_string());
        for turn in conversation_history {
            let role = match turn.role {
                ConversationRole::Candidate => "Candidate",
                ConversationRole::Assistant => "Assistant",
            };
            parts.push(format!("  [{role}]: {}", turn.content));
        }
    }

    parts.push(format!("\nCurrent message:\n{raw_text}"));
    parts.push("\nExtract any available: candidate name, skills, years of experience, \
         education, current company, desired salary, location, and position title. \
         Also note any observations about the candidate and any facts that need confirmation.".to_string());

    parts.join("\n")
}

/// Build the responder input from the service state.
pub fn build_responder_input(
    candidate_id: &str,
    position_url: &str,
    company_name: Option<&str>,
    profile_summary: &str,
    conversation_history: &[ConversationTurn],
) -> ResponderInput {
    ResponderInput {
        candidate_id: candidate_id.to_string(),
        position_url: position_url.to_string(),
        company_name: company_name.map(|s| s.to_string()),
        candidate_profile_summary: profile_summary.to_string(),
        conversation_history: conversation_history.to_vec(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::agent::types::ConversationRole;

    #[test]
    fn build_extraction_prompt_includes_raw_text() {
        let prompt = build_extraction_prompt(
            "John has 5 years of Rust experience",
            "",
            &[],
        );
        assert!(prompt.contains("John has 5 years of Rust experience"));
        assert!(prompt.contains("Extract"));
    }

    #[test]
    fn build_extraction_prompt_includes_profile_summary() {
        let prompt = build_extraction_prompt(
            "Hello",
            "Name: Alice; Skills: Rust",
            &[],
        );
        assert!(prompt.contains("Alice"));
        assert!(prompt.contains("Current profile"));
    }

    #[test]
    fn build_extraction_prompt_includes_conversation_history() {
        let history = vec![ConversationTurn {
            role: ConversationRole::Candidate,
            content: "I know Python".to_string(),
        }];
        let prompt = build_extraction_prompt(
            "And Rust too",
            "",
            &history,
        );
        assert!(prompt.contains("I know Python"));
        assert!(prompt.contains("Recent conversation"));
    }

    #[test]
    fn build_responder_input_assembles_fields() {
        let input = build_responder_input(
            "user-1",
            "https://example.com/job/1",
            Some("Acme Corp"),
            "Profile summary",
            &[ConversationTurn {
                role: ConversationRole::Candidate,
                content: "Hello".to_string(),
            }],
        );
        assert_eq!(input.candidate_id, "user-1");
        assert_eq!(input.position_url, "https://example.com/job/1");
        assert_eq!(input.company_name, Some("Acme Corp".to_string()));
        assert_eq!(input.candidate_profile_summary, "Profile summary");
        assert_eq!(input.conversation_history.len(), 1);
    }

    #[test]
    fn build_responder_input_handles_none_company() {
        let input = build_responder_input(
            "user-1",
            "url",
            None,
            "summary",
            &[],
        );
        assert!(input.company_name.is_none());
    }
}
