use std::fmt;
use std::fs;
use std::path::Path;

use serde::Deserialize;

const DEFAULT_CONFIG_YAML: &str =
    "browser_exe_path: \"\"\nuser_data_dir: \"default\"\nqr_output_path: \"qr_code.png\"\n";
const DEFAULT_USER_DATA_DIR: &str = "default";
const DEFAULT_QR_OUTPUT_PATH: &str = "qr_code.png";

#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub struct AppConfig {
    pub browser_exe_path: Option<String>,
    pub user_data_dir: Option<String>,
    pub qr_output_path: Option<String>,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            browser_exe_path: None,
            user_data_dir: Some(DEFAULT_USER_DATA_DIR.to_string()),
            qr_output_path: Some(DEFAULT_QR_OUTPUT_PATH.to_string()),
        }
    }
}

impl AppConfig {
    pub fn browser_exe_path(&self) -> Option<&str> {
        self.browser_exe_path
            .as_deref()
            .filter(|value| !value.is_empty())
    }

    pub fn user_data_dir(&self) -> &str {
        match self.user_data_dir.as_deref() {
            Some("") | None => DEFAULT_USER_DATA_DIR,
            Some(value) => value,
        }
    }

    pub fn qr_output_path(&self) -> &str {
        match self.qr_output_path.as_deref() {
            Some("") | None => DEFAULT_QR_OUTPUT_PATH,
            Some(value) => value,
        }
    }
}

#[derive(Debug)]
pub enum ConfigError {
    Io(std::io::Error),
    Parse(serde_yaml::Error),
}

impl fmt::Display for ConfigError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io(err) => write!(f, "failed to access config file: {err}"),
            Self::Parse(err) => write!(f, "failed to parse config yaml: {err}"),
        }
    }
}

impl std::error::Error for ConfigError {}

impl From<std::io::Error> for ConfigError {
    fn from(value: std::io::Error) -> Self {
        Self::Io(value)
    }
}

impl From<serde_yaml::Error> for ConfigError {
    fn from(value: serde_yaml::Error) -> Self {
        Self::Parse(value)
    }
}

pub fn load_or_create(path: impl AsRef<Path>) -> Result<AppConfig, ConfigError> {
    let path = path.as_ref();

    if !path.exists() {
        if let Some(parent) = path
            .parent()
            .filter(|parent| !parent.as_os_str().is_empty())
        {
            fs::create_dir_all(parent)?;
        }
        fs::write(path, DEFAULT_CONFIG_YAML)?;
    }

    let content = fs::read_to_string(path)?;
    let config = serde_yaml::from_str::<AppConfig>(&content)?;

    Ok(config)
}

#[cfg(test)]
mod tests {
    use std::time::{SystemTime, UNIX_EPOCH};

    use super::*;

    #[test]
    fn creates_default_config_when_file_is_missing() {
        let dir = unique_temp_dir("missing_config");
        let path = dir.join("config.yaml");

        let config = load_or_create(&path).expect("missing config should be created");

        assert!(path.exists());
        assert_eq!(config.browser_exe_path(), None);
        assert_eq!(config.user_data_dir(), DEFAULT_USER_DATA_DIR);
    }

    #[test]
    fn returns_none_for_empty_browser_exe_path() {
        let dir = unique_temp_dir("empty_browser");
        let path = dir.join("config.yaml");

        write_config(&path, "browser_exe_path: \"\"\nuser_data_dir: \"custom\"\n");

        let config = load_or_create(&path).expect("config should load");

        assert_eq!(config.browser_exe_path(), None);
    }

    #[test]
    fn returns_default_qr_output_path_when_missing() {
        let dir = unique_temp_dir("missing_qr_output_path");
        let path = dir.join("config.yaml");

        write_config(
            &path,
            "browser_exe_path: \"\"\nuser_data_dir: \"custom-session\"\n",
        );

        let config = load_or_create(&path).expect("config should load");

        assert_eq!(config.qr_output_path(), "qr_code.png");
    }

    #[test]
    fn returns_explicit_qr_output_path_when_present() {
        let dir = unique_temp_dir("explicit_qr_output_path");
        let path = dir.join("config.yaml");

        write_config(
            &path,
            "browser_exe_path: \"\"\nuser_data_dir: \"/data/session\"\nqr_output_path: \"/data/qr/qr_code.png\"\n",
        );

        let config = load_or_create(&path).expect("config should load");

        assert_eq!(config.qr_output_path(), "/data/qr/qr_code.png");
    }

    #[test]
    fn returns_default_for_missing_or_empty_user_data_dir() {
        let missing_dir = unique_temp_dir("missing_user_data_dir");
        let missing_path = missing_dir.join("config.yaml");
        write_config(&missing_path, "browser_exe_path: \"/browser\"\n");

        let missing_config = load_or_create(&missing_path).expect("config should load");
        assert_eq!(missing_config.user_data_dir(), DEFAULT_USER_DATA_DIR);

        let empty_dir = unique_temp_dir("empty_user_data_dir");
        let empty_path = empty_dir.join("config.yaml");
        write_config(
            &empty_path,
            "browser_exe_path: \"/browser\"\nuser_data_dir: \"\"\n",
        );

        let empty_config = load_or_create(&empty_path).expect("config should load");
        assert_eq!(empty_config.user_data_dir(), DEFAULT_USER_DATA_DIR);
    }

    #[test]
    fn returns_error_for_invalid_yaml() {
        let dir = unique_temp_dir("invalid_yaml");
        let path = dir.join("config.yaml");
        write_config(&path, "browser_exe_path: [broken\n");

        let result = load_or_create(&path);

        assert!(matches!(result, Err(ConfigError::Parse(_))));
    }

    fn unique_temp_dir(label: &str) -> std::path::PathBuf {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system clock should be after unix epoch")
            .as_nanos();
        let dir =
            std::env::temp_dir().join(format!("boss_mcp_{label}_{}_{}", std::process::id(), nanos));
        fs::create_dir_all(&dir).expect("temp dir should be created");
        dir
    }

    fn write_config(path: &Path, content: &str) {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).expect("config parent directory should exist");
        }
        fs::write(path, content).expect("config file should be written");
    }
}
