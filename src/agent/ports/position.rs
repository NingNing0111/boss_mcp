use async_trait::async_trait;

use crate::agent::error::AgentError;

/// Position details retrieved from an external source (e.g., Boss).
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq)]
pub struct PositionInfo {
    pub title: String,
    pub salary: Option<String>,
    pub description: Option<String>,
    pub requirements: Vec<String>,
    pub keywords: Vec<String>,
}

/// Port for looking up position information.
#[async_trait]
pub trait PositionLookup: Send + Sync {
    async fn lookup(&self, position_url: &str) -> std::result::Result<Option<PositionInfo>, AgentError>;
}

#[cfg(test)]
mod tests {
    use super::*;

    pub struct FakePositionLookup {
        pub result: std::result::Result<Option<PositionInfo>, AgentError>,
    }

    impl FakePositionLookup {
        pub fn returning(info: Option<PositionInfo>) -> Self {
            Self { result: Ok(info) }
        }

        pub fn failing(error: AgentError) -> Self {
            Self { result: Err(error) }
        }
    }

    #[async_trait]
    impl PositionLookup for FakePositionLookup {
        async fn lookup(&self, _position_url: &str) -> std::result::Result<Option<PositionInfo>, AgentError> {
            self.result.clone()
        }
    }

    #[tokio::test]
    async fn fake_position_lookup_returns_preset() {
        let info = PositionInfo {
            title: "Senior Rust Developer".to_string(),
            salary: Some("30-50K".to_string()),
            description: Some("Build great software".to_string()),
            requirements: vec!["5 years Rust".to_string()],
            keywords: vec!["rust".to_string()],
        };
        let lookup = FakePositionLookup::returning(Some(info.clone()));
        let result = lookup.lookup("https://example.com/job/1").await.expect("should succeed");
        assert_eq!(result, Some(info));
    }
}
