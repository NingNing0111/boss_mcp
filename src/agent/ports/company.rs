use async_trait::async_trait;

use crate::agent::error::AgentError;

/// Company information retrieved from an external source (e.g., QCC).
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq)]
pub struct CompanyInfo {
    pub name: String,
    pub industry: Option<String>,
    pub scale: Option<String>,
    pub description: Option<String>,
}

/// Port for looking up company information.
#[async_trait]
pub trait CompanyLookup: Send + Sync {
    async fn lookup(&self, company_name: &str) -> std::result::Result<Option<CompanyInfo>, AgentError>;
}

#[cfg(test)]
mod tests {
    use super::*;

    pub struct FakeCompanyLookup {
        pub result: std::result::Result<Option<CompanyInfo>, AgentError>,
    }

    impl FakeCompanyLookup {
        pub fn returning(info: Option<CompanyInfo>) -> Self {
            Self { result: Ok(info) }
        }

        pub fn failing(error: AgentError) -> Self {
            Self { result: Err(error) }
        }
    }

    #[async_trait]
    impl CompanyLookup for FakeCompanyLookup {
        async fn lookup(&self, _company_name: &str) -> std::result::Result<Option<CompanyInfo>, AgentError> {
            self.result.clone()
        }
    }

    #[tokio::test]
    async fn fake_company_lookup_returns_preset() {
        let info = CompanyInfo {
            name: "Test Corp".to_string(),
            industry: Some("Tech".to_string()),
            scale: None,
            description: None,
        };
        let lookup = FakeCompanyLookup::returning(Some(info.clone()));
        let result = lookup.lookup("Test Corp").await.expect("should succeed");
        assert_eq!(result, Some(info));
    }

    #[tokio::test]
    async fn fake_company_lookup_returns_none() {
        let lookup = FakeCompanyLookup::returning(None);
        let result = lookup.lookup("Unknown").await.expect("should succeed");
        assert!(result.is_none());
    }
}
