use async_trait::async_trait;

use crate::agent::error::AgentError;
use crate::agent::types::{ExtractionContext, ExtractedInfo};

/// Trait for extracting candidate information from text using LLM.
#[async_trait]
pub trait Extractor: Send + Sync {
    /// Extract structured candidate information from the given extraction context.
    async fn extract(&self, context: &ExtractionContext) -> std::result::Result<ExtractedInfo, AgentError>;
}

/// Rig-based extractor that uses the rig-core extraction pipeline.
pub struct RigExtractor {
    api_key: String,
    base_url: String,
    model_name: String,
}

impl RigExtractor {
    pub fn new(api_key: impl Into<String>, base_url: impl Into<String>, model_name: impl Into<String>) -> Self {
        Self {
            api_key: api_key.into(),
            base_url: base_url.into(),
            model_name: model_name.into(),
        }
    }
}

#[async_trait]
impl Extractor for RigExtractor {
    async fn extract(&self, context: &ExtractionContext) -> std::result::Result<ExtractedInfo, AgentError> {
        use rig::providers::openai;

        let client = openai::Client::from_url(&self.api_key, &self.base_url);
        let extractor = client
            .extractor::<ExtractedInfo>(&self.model_name)
            .build();

        // Build extraction prompt from context
        let prompt = crate::agent::llm::prompt::build_extraction_prompt(
            &context.user_message,
            &context.profile_summary,
            &context.conversation_history,
        );

        let result = extractor
            .extract(&prompt)
            .await
            .map_err(|e| AgentError::Extraction(e.to_string()))?;

        Ok(result)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A deterministic fake extractor for testing.
    pub struct FakeExtractor {
        pub result: std::result::Result<ExtractedInfo, AgentError>,
    }

    impl FakeExtractor {
        pub fn returning(info: ExtractedInfo) -> Self {
            Self {
                result: Ok(info),
            }
        }

        pub fn failing(error: AgentError) -> Self {
            Self {
                result: Err(error),
            }
        }
    }

    #[async_trait]
    impl Extractor for FakeExtractor {
        async fn extract(&self, _context: &ExtractionContext) -> std::result::Result<ExtractedInfo, AgentError> {
            self.result.clone()
        }
    }

    #[tokio::test]
    async fn fake_extractor_returns_preset_result() {
        let info = ExtractedInfo {
            candidate_name: Some("Test".to_string()),
            ..Default::default()
        };
        let extractor = FakeExtractor::returning(info.clone());
        let context = ExtractionContext {
            user_message: "any text".to_string(),
            profile_summary: String::new(),
            conversation_history: Vec::new(),
        };
        let result = extractor.extract(&context).await.expect("should succeed");
        assert_eq!(result, info);
    }

    #[tokio::test]
    async fn fake_extractor_returns_preset_error() {
        let extractor = FakeExtractor::failing(AgentError::Extraction("test error".to_string()));
        let context = ExtractionContext {
            user_message: "any text".to_string(),
            profile_summary: String::new(),
            conversation_history: Vec::new(),
        };
        let result = extractor.extract(&context).await;
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("test error"));
    }
}
