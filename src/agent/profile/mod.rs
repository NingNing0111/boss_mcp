pub mod merge;
pub mod model;
pub mod repository;

pub use merge::merge_profile;
pub use model::CandidateProfile;
pub use repository::{FileProfileRepository, ProfileRepository};
