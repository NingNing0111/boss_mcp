use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use crate::agent::types::SCHEMA_VERSION;

/// A candidate profile with confirmed facts and metadata.
/// Confirmed facts are authoritative and cannot be overridden by candidate facts.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CandidateProfile {
    /// Schema version for migration support.
    pub schema_version: String,

    /// Unique candidate identifier.
    pub candidate_id: String,

    /// Confirmed name (from manual verification or trusted source).
    #[serde(default)]
    pub confirmed_name: Option<String>,

    /// Candidate-provided or extracted name.
    #[serde(default)]
    pub candidate_name: Option<String>,

    /// Confirmed skills list (manually verified).
    #[serde(default)]
    pub confirmed_skills: Vec<String>,

    /// Candidate-provided or extracted skills.
    #[serde(default)]
    pub candidate_skills: Vec<String>,

    /// Confirmed years of experience.
    #[serde(default)]
    pub confirmed_experience_years: Option<u32>,

    /// Candidate-provided years of experience.
    #[serde(default)]
    pub candidate_experience_years: Option<u32>,

    /// Confirmed education.
    #[serde(default)]
    pub confirmed_education: Option<String>,

    /// Candidate-provided education.
    #[serde(default)]
    pub candidate_education: Option<String>,

    /// Confirmed current company.
    #[serde(default)]
    pub confirmed_current_company: Option<String>,

    /// Candidate-provided current company.
    #[serde(default)]
    pub candidate_current_company: Option<String>,

    /// Desired salary range.
    #[serde(default)]
    pub desired_salary: Option<String>,

    /// Location preference.
    #[serde(default)]
    pub location: Option<String>,

    /// Position title of interest.
    #[serde(default)]
    pub position_title: Option<String>,

    /// Timestamp when this profile was first created.
    pub created_at: DateTime<Utc>,

    /// Timestamp when this profile was last updated.
    pub updated_at: DateTime<Utc>,
}

impl CandidateProfile {
    /// Create a new empty profile for the given candidate ID.
    pub fn new(candidate_id: impl Into<String>) -> Self {
        let now = Utc::now();
        Self {
            schema_version: SCHEMA_VERSION.to_string(),
            candidate_id: candidate_id.into(),
            confirmed_name: None,
            candidate_name: None,
            confirmed_skills: Vec::new(),
            candidate_skills: Vec::new(),
            confirmed_experience_years: None,
            candidate_experience_years: None,
            confirmed_education: None,
            candidate_education: None,
            confirmed_current_company: None,
            candidate_current_company: None,
            desired_salary: None,
            location: None,
            position_title: None,
            created_at: now,
            updated_at: now,
        }
    }

    /// Deterministic file name for persisting this profile.
    pub fn file_name(&self) -> String {
        format!("{}.json", sanitize_filename(&self.candidate_id))
    }

    /// Build a human-readable summary of the profile for the responder.
    pub fn summary(&self) -> String {
        let mut parts = Vec::new();

        if let Some(name) = self.effective_name() {
            parts.push(format!("Name: {name}"));
        }
        if let Some(exp) = self.effective_experience_years() {
            parts.push(format!("Experience: {exp} years"));
        }
        if let Some(edu) = self.effective_education() {
            parts.push(format!("Education: {edu}"));
        }
        if let Some(company) = self.effective_current_company() {
            parts.push(format!("Current company: {company}"));
        }
        if let Some(salary) = &self.desired_salary {
            parts.push(format!("Desired salary: {salary}"));
        }
        if let Some(loc) = &self.location {
            parts.push(format!("Location: {loc}"));
        }
        if let Some(pos) = &self.position_title {
            parts.push(format!("Position: {pos}"));
        }

        let skills = self.effective_skills();
        if !skills.is_empty() {
            parts.push(format!("Skills: {}", skills.join(", ")));
        }

        if parts.is_empty() {
            "No profile information available.".to_string()
        } else {
            parts.join("; ")
        }
    }

    /// Effective name: confirmed outranks candidate.
    pub fn effective_name(&self) -> Option<&str> {
        self.confirmed_name
            .as_deref()
            .filter(|s| !s.is_empty())
            .or_else(|| {
                self.candidate_name
                    .as_deref()
                    .filter(|s| !s.is_empty())
            })
    }

    /// Effective experience years: confirmed outranks candidate.
    pub fn effective_experience_years(&self) -> Option<u32> {
        self.confirmed_experience_years.or(self.candidate_experience_years)
    }

    /// Effective education: confirmed outranks candidate.
    pub fn effective_education(&self) -> Option<&str> {
        self.confirmed_education
            .as_deref()
            .filter(|s| !s.is_empty())
            .or_else(|| {
                self.candidate_education
                    .as_deref()
                    .filter(|s| !s.is_empty())
            })
    }

    /// Effective current company: confirmed outranks candidate.
    pub fn effective_current_company(&self) -> Option<&str> {
        self.confirmed_current_company
            .as_deref()
            .filter(|s| !s.is_empty())
            .or_else(|| {
                self.candidate_current_company
                    .as_deref()
                    .filter(|s| !s.is_empty())
            })
    }

    /// Effective skills: union of confirmed and candidate, deduped.
    pub fn effective_skills(&self) -> Vec<&str> {
        let mut seen = std::collections::HashSet::new();
        let mut result = Vec::new();

        for skill in self.confirmed_skills.iter().chain(self.candidate_skills.iter()) {
            let lower = skill.to_lowercase();
            if seen.insert(lower) {
                result.push(skill.as_str());
            }
        }

        result
    }
}

/// Sanitize a candidate ID for use as a filename.
/// Replaces path separators and other unsafe characters.
fn sanitize_filename(input: &str) -> String {
    input
        .replace(['/', '\\', ':', '*', '?', '"', '<', '>', '|'], "_")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_profile_has_empty_fields() {
        let profile = CandidateProfile::new("user-1");
        assert_eq!(profile.candidate_id, "user-1");
        assert_eq!(profile.schema_version, SCHEMA_VERSION);
        assert!(profile.confirmed_name.is_none());
        assert!(profile.candidate_skills.is_empty());
        assert!(profile.effective_skills().is_empty());
    }

    #[test]
    fn confirmed_name_outranks_candidate_name() {
        let mut profile = CandidateProfile::new("user-1");
        profile.candidate_name = Some("Alice".to_string());
        assert_eq!(profile.effective_name(), Some("Alice"));

        profile.confirmed_name = Some("Alicia".to_string());
        assert_eq!(profile.effective_name(), Some("Alicia"));
    }

    #[test]
    fn empty_confirmed_name_falls_through_to_candidate() {
        let mut profile = CandidateProfile::new("user-1");
        profile.confirmed_name = Some(String::new());
        profile.candidate_name = Some("Bob".to_string());
        assert_eq!(profile.effective_name(), Some("Bob"));
    }

    #[test]
    fn effective_skills_dedupes_case_insensitively() {
        let mut profile = CandidateProfile::new("user-1");
        profile.confirmed_skills = vec!["Rust".to_string(), "Python".to_string()];
        profile.candidate_skills = vec!["rust".to_string(), "Go".to_string()];
        let skills = profile.effective_skills();
        assert_eq!(skills, vec!["Rust", "Python", "Go"]);
    }

    #[test]
    fn summary_contains_key_fields() {
        let mut profile = CandidateProfile::new("user-1");
        profile.candidate_name = Some("Charlie".to_string());
        profile.candidate_experience_years = Some(5);
        profile.location = Some("Beijing".to_string());

        let summary = profile.summary();
        assert!(summary.contains("Charlie"));
        assert!(summary.contains("5 years"));
        assert!(summary.contains("Beijing"));
    }

    #[test]
    fn summary_shows_no_info_when_empty() {
        let profile = CandidateProfile::new("user-1");
        assert_eq!(profile.summary(), "No profile information available.");
    }

    #[test]
    fn file_name_sanitizes_special_chars() {
        let profile = CandidateProfile::new("user/123:456");
        assert_eq!(profile.file_name(), "user_123_456.json");
    }

    #[test]
    fn timestamps_are_set_on_creation() {
        let profile = CandidateProfile::new("user-1");
        assert!(profile.created_at <= Utc::now());
        assert_eq!(profile.created_at, profile.updated_at);
    }
}
