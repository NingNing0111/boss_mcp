use chrono::Utc;

use crate::agent::profile::model::CandidateProfile;
use crate::agent::types::ExtractedInfo;

/// Merge candidate facts from the extractor into the existing profile.
///
/// Rules:
/// - Confirmed facts are never overwritten.
/// - Empty values in the new data do not erase existing non-empty values.
/// - List fields are unioned and deduped (case-insensitive).
/// - The `updated_at` timestamp is always refreshed.
/// - Returns a new profile; does not mutate the input.
pub fn merge_profile(
    existing: &CandidateProfile,
    extracted: &ExtractedInfo,
) -> CandidateProfile {
    let mut merged = existing.clone();

    // Scalar fields: only update if new value is non-empty and no confirmed value exists
    if let Some(ref name) = extracted.candidate_name {
        if !name.trim().is_empty() && existing.confirmed_name.is_none() {
            merged.candidate_name = Some(name.clone());
        }
    }

    if let Some(years) = extracted.experience_years {
        if existing.confirmed_experience_years.is_none() {
            merged.candidate_experience_years = Some(years);
        }
    }

    if let Some(ref edu) = extracted.education {
        if !edu.trim().is_empty() && existing.confirmed_education.is_none() {
            merged.candidate_education = Some(edu.clone());
        }
    }

    if let Some(ref company) = extracted.current_company {
        if !company.trim().is_empty() && existing.confirmed_current_company.is_none() {
            merged.candidate_current_company = Some(company.clone());
        }
    }

    if let Some(ref salary) = extracted.desired_salary {
        if !salary.trim().is_empty() && merged.desired_salary.is_none() {
            merged.desired_salary = Some(salary.clone());
        }
    }

    if let Some(ref location) = extracted.location {
        if !location.trim().is_empty() && merged.location.is_none() {
            merged.location = Some(location.clone());
        }
    }

    if let Some(ref title) = extracted.position_title {
        if !title.trim().is_empty() && merged.position_title.is_none() {
            merged.position_title = Some(title.clone());
        }
    }

    // List fields: union and dedupe case-insensitively
    merged.candidate_skills = union_skills(
        &existing.confirmed_skills,
        &existing.candidate_skills,
        &extracted.skills,
    );

    merged.updated_at = Utc::now();

    merged
}

/// Compute the union of confirmed + existing candidate + new candidate skills,
/// deduped case-insensitively. Confirmed skills always appear first.
fn union_skills(
    confirmed: &[String],
    existing_candidate: &[String],
    new_candidate: &[String],
) -> Vec<String> {
    let mut seen = std::collections::HashSet::new();
    let mut result = Vec::new();

    for skill in confirmed.iter().chain(existing_candidate.iter()).chain(new_candidate.iter()) {
        let lower = skill.to_lowercase();
        if seen.insert(lower) {
            result.push(skill.clone());
        }
    }

    result
}

#[cfg(test)]
mod tests {
    use super::*;

    fn base_profile() -> CandidateProfile {
        CandidateProfile::new("user-1")
    }

    #[test]
    fn merge_updates_candidate_name_when_no_confirmed() {
        let profile = base_profile();
        let extracted = ExtractedInfo {
            candidate_name: Some("Alice".to_string()),
            ..Default::default()
        };
        let merged = merge_profile(&profile, &extracted);
        assert_eq!(merged.candidate_name, Some("Alice".to_string()));
    }

    #[test]
    fn merge_does_not_overwrite_confirmed_name() {
        let mut profile = base_profile();
        profile.confirmed_name = Some("Alicia".to_string());

        let extracted = ExtractedInfo {
            candidate_name: Some("Alice".to_string()),
            ..Default::default()
        };
        let merged = merge_profile(&profile, &extracted);
        assert_eq!(merged.confirmed_name, Some("Alicia".to_string()));
        assert_eq!(merged.candidate_name, None);
    }

    #[test]
    fn merge_does_not_erase_with_empty_string() {
        let mut profile = base_profile();
        profile.candidate_name = Some("Bob".to_string());

        let extracted = ExtractedInfo {
            candidate_name: Some("".to_string()),
            ..Default::default()
        };
        let merged = merge_profile(&profile, &extracted);
        assert_eq!(merged.candidate_name, Some("Bob".to_string()));
    }

    #[test]
    fn merge_does_not_erase_with_whitespace_only() {
        let mut profile = base_profile();
        profile.candidate_name = Some("Bob".to_string());

        let extracted = ExtractedInfo {
            candidate_name: Some("   ".to_string()),
            ..Default::default()
        };
        let merged = merge_profile(&profile, &extracted);
        assert_eq!(merged.candidate_name, Some("Bob".to_string()));
    }

    #[test]
    fn merge_unions_skills_case_insensitively() {
        let mut profile = base_profile();
        profile.confirmed_skills = vec!["Rust".to_string()];
        profile.candidate_skills = vec!["Python".to_string()];

        let extracted = ExtractedInfo {
            skills: vec!["rust".to_string(), "Go".to_string()],
            ..Default::default()
        };
        let merged = merge_profile(&profile, &extracted);
        assert_eq!(merged.candidate_skills, vec!["Rust", "Python", "Go"]);
    }

    #[test]
    fn merge_preserves_existing_desired_salary_when_new_is_empty() {
        let mut profile = base_profile();
        profile.desired_salary = Some("20-30K".to_string());

        let extracted = ExtractedInfo {
            desired_salary: Some("".to_string()),
            ..Default::default()
        };
        let merged = merge_profile(&profile, &extracted);
        assert_eq!(merged.desired_salary, Some("20-30K".to_string()));
    }

    #[test]
    fn merge_updates_desired_salary_when_existing_is_none() {
        let profile = base_profile();
        let extracted = ExtractedInfo {
            desired_salary: Some("25-35K".to_string()),
            ..Default::default()
        };
        let merged = merge_profile(&profile, &extracted);
        assert_eq!(merged.desired_salary, Some("25-35K".to_string()));
    }

    #[test]
    fn merge_does_not_overwrite_desired_salary_when_existing_has_value() {
        let mut profile = base_profile();
        profile.desired_salary = Some("20-30K".to_string());

        let extracted = ExtractedInfo {
            desired_salary: Some("25-35K".to_string()),
            ..Default::default()
        };
        let merged = merge_profile(&profile, &extracted);
        // existing already has a value, so it stays
        assert_eq!(merged.desired_salary, Some("20-30K".to_string()));
    }

    #[test]
    fn merge_updates_experience_years_when_no_confirmed() {
        let profile = base_profile();
        let extracted = ExtractedInfo {
            experience_years: Some(5),
            ..Default::default()
        };
        let merged = merge_profile(&profile, &extracted);
        assert_eq!(merged.candidate_experience_years, Some(5));
    }

    #[test]
    fn merge_does_not_overwrite_confirmed_experience_years() {
        let mut profile = base_profile();
        profile.confirmed_experience_years = Some(3);

        let extracted = ExtractedInfo {
            experience_years: Some(10),
            ..Default::default()
        };
        let merged = merge_profile(&profile, &extracted);
        assert_eq!(merged.confirmed_experience_years, Some(3));
        assert_eq!(merged.candidate_experience_years, None);
    }

    #[test]
    fn merge_updates_location_when_existing_is_none() {
        let profile = base_profile();
        let extracted = ExtractedInfo {
            location: Some("Shanghai".to_string()),
            ..Default::default()
        };
        let merged = merge_profile(&profile, &extracted);
        assert_eq!(merged.location, Some("Shanghai".to_string()));
    }

    #[test]
    fn merge_refreshes_updated_at() {
        let profile = base_profile();
        let before = profile.updated_at;

        let extracted = ExtractedInfo {
            candidate_name: Some("Test".to_string()),
            ..Default::default()
        };
        let merged = merge_profile(&profile, &extracted);
        assert!(merged.updated_at >= before);
    }

    #[test]
    fn merge_does_not_mutate_input() {
        let mut profile = base_profile();
        profile.candidate_name = Some("Original".to_string());
        let profile_snapshot = profile.clone();

        let extracted = ExtractedInfo {
            candidate_name: Some("Updated".to_string()),
            ..Default::default()
        };
        let _merged = merge_profile(&profile, &extracted);

        assert_eq!(profile, profile_snapshot, "original profile should not be mutated");
    }

    #[test]
    fn merge_handles_empty_extracted_info() {
        let mut profile = base_profile();
        profile.candidate_name = Some("Alice".to_string());

        let extracted = ExtractedInfo::default();
        let merged = merge_profile(&profile, &extracted);

        assert_eq!(merged.candidate_name, Some("Alice".to_string()));
    }

    #[test]
    fn merge_handles_large_skill_lists() {
        let mut profile = base_profile();
        profile.confirmed_skills = (0..500).map(|i| format!("skill-confirmed-{i}")).collect();
        profile.candidate_skills = (0..500).map(|i| format!("skill-candidate-{i}")).collect();

        let extracted = ExtractedInfo {
            skills: (0..500).map(|i| format!("skill-new-{i}")).collect(),
            ..Default::default()
        };
        let merged = merge_profile(&profile, &extracted);

        assert_eq!(merged.candidate_skills.len(), 1500);
    }
}
