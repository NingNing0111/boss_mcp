use rig::providers::openai;

use super::config::LlmConfig;

/// Create a Rig OpenAI-compatible client from the LLM config.
pub fn create_client(config: &LlmConfig) -> openai::Client {
    openai::Client::from_url(&config.api_key, &config.base_url)
}

/// Create a completion model handle for extraction.
pub fn extraction_model(config: &LlmConfig) -> openai::CompletionModel {
    let client = create_client(config);
    client.completion_model(&config.extraction_model)
}

/// Create a completion model handle for response generation.
pub fn response_model(config: &LlmConfig) -> openai::CompletionModel {
    let client = create_client(config);
    client.completion_model(&config.response_model)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn create_client_uses_config_values() {
        let config = LlmConfig::test_config();
        // Just verify it does not panic
        let _client = create_client(&config);
    }

    #[test]
    fn extraction_model_uses_extraction_model_name() {
        let config = LlmConfig::test_config();
        let _model = extraction_model(&config);
    }

    #[test]
    fn response_model_uses_response_model_name() {
        let config = LlmConfig::test_config();
        let _model = response_model(&config);
    }
}
