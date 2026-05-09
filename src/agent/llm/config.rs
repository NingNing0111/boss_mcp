/// LLM configuration for the agent service.
#[derive(Debug, Clone)]
pub struct LlmConfig {
    /// OpenAI-compatible API key.
    pub api_key: String,
    /// Base URL for the API. Should include /v1 if the gateway expects it.
    pub base_url: String,
    /// Model name for extraction (e.g., "gpt-4o").
    pub extraction_model: String,
    /// Model name for response generation (e.g., "gpt-4o").
    pub response_model: String,
}

impl LlmConfig {
    /// Create config from environment variables with defaults.
    pub fn from_env() -> Option<Self> {
        let api_key = std::env::var("OPENAI_API_KEY").ok()?;
        let base_url = std::env::var("OPENAI_BASE_URL")
            .unwrap_or_else(|_| "https://api.openai.com/v1".to_string());
        let extraction_model = std::env::var("AGENT_EXTRACTION_MODEL")
            .unwrap_or_else(|_| "gpt-4o".to_string());
        let response_model = std::env::var("AGENT_RESPONSE_MODEL")
            .unwrap_or_else(|_| "gpt-4o".to_string());

        Some(Self {
            api_key,
            base_url,
            extraction_model,
            response_model,
        })
    }

    /// Create a test config with dummy values.
    #[cfg(test)]
    pub fn test_config() -> Self {
        Self {
            api_key: "test-key".to_string(),
            base_url: "https://api.example.com/v1".to_string(),
            extraction_model: "test-model".to_string(),
            response_model: "test-model".to_string(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_has_expected_defaults() {
        let config = LlmConfig::test_config();
        assert_eq!(config.api_key, "test-key");
        assert_eq!(config.base_url, "https://api.example.com/v1");
        assert_eq!(config.extraction_model, "test-model");
        assert_eq!(config.response_model, "test-model");
    }
}
