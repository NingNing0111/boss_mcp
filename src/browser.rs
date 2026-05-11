use std::fmt;
use std::sync::{Arc, Mutex, OnceLock};

use anyhow::Result as AnyhowResult;
use rust_drission::{BrowserConfig, ChromiumPage, Page};

use crate::config::AppConfig;

static BROWSER_STATE: BrowserState = BrowserState::new();

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BrowserError {
    NotConfigured,
    InitFailed(String),
    LockPoisoned,
    OperationFailed(String),
}

impl fmt::Display for BrowserError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NotConfigured => write!(f, "browser config has not been initialized"),
            Self::InitFailed(message) => write!(f, "failed to initialize browser: {message}"),
            Self::LockPoisoned => write!(f, "browser lock was poisoned"),
            Self::OperationFailed(message) => write!(f, "browser operation failed: {message}"),
        }
    }
}

impl std::error::Error for BrowserError {}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TabType {
    Boss,
    Qcc,
}

pub fn init(config: AppConfig) -> Result<(), BrowserError> {
    BROWSER_STATE.init(config, create_browser)
}

pub fn with_tab<T, F>(tab: TabType, op: F) -> AnyhowResult<T>
where
    F: FnOnce(&Page) -> AnyhowResult<T>,
{
    BROWSER_STATE.with_tab(tab, op, create_browser)
}

pub fn with_boss_tab<T, F>(op: F) -> AnyhowResult<T>
where
    F: FnOnce(&Page) -> AnyhowResult<T>,
{
    with_tab(TabType::Boss, op)
}

pub fn with_qcc_tab<T, F>(op: F) -> AnyhowResult<T>
where
    F: FnOnce(&Page) -> AnyhowResult<T>,
{
    with_tab(TabType::Qcc, op)
}

pub fn with_new_tab<T, F>(op: F) -> AnyhowResult<T>
where
    F: FnOnce(&Page) -> AnyhowResult<T>,
{
    BROWSER_STATE.with_new_tab(op, create_browser)
}

fn create_browser(config: &AppConfig) -> Result<BrowserTabs, BrowserError> {
    let browser_config = build_browser_config(config);

    let page = ChromiumPage::new(browser_config).map_err(|err| BrowserError::InitFailed(err.to_string()))?;
    let boss_tab = page.tab().to_owned();
    let qcc_tab = page.new_tab(None).map_err(|err| BrowserError::InitFailed(err.to_string()))?;

    Ok(BrowserTabs {
        chromium_page: page,
        boss_tab,
        qcc_tab,
    })
}

struct BrowserTabs {
    chromium_page: ChromiumPage,
    boss_tab: Page,
    qcc_tab: Page,
}

fn build_browser_config(config: &AppConfig) -> BrowserConfig {
    let user_data = config.user_data_dir();
    println!("{}", user_data);
    let mut browser_config = BrowserConfig::new()
        .headless(false)
        .user_data_dir(user_data);

    if let Some(path) = config.browser_exe_path() {
        browser_config = browser_config.chrome_path(path);
    }

    browser_config
}

struct BrowserState {
    config: OnceLock<AppConfig>,
    tabs: Mutex<Option<Arc<Mutex<BrowserTabs>>>>,
}

impl BrowserState {
    const fn new() -> Self {
        Self {
            config: OnceLock::new(),
            tabs: Mutex::new(None),
        }
    }

    fn init<F>(&self, config: AppConfig, factory: F) -> Result<(), BrowserError>
    where
        F: Fn(&AppConfig) -> Result<BrowserTabs, BrowserError>,
    {
        let _ = self.config.set(config);
        let _ = self.ensure_tabs(&factory)?;
        Ok(())
    }

    fn with_tab<R, Op, F>(&self, tab_type: TabType, op: Op, factory: F) -> AnyhowResult<R>
    where
        Op: FnOnce(&Page) -> AnyhowResult<R>,
        F: Fn(&AppConfig) -> Result<BrowserTabs, BrowserError>,
    {
        let tabs = self.ensure_tabs(&factory)?;
        let guard = tabs.lock().map_err(|_| BrowserError::LockPoisoned)?;
        let tab = match tab_type {
            TabType::Boss => &guard.boss_tab,
            TabType::Qcc => &guard.qcc_tab,
        };
        op(tab)
    }

    fn with_new_tab<R, Op, F>(&self, op: Op, factory: F) -> AnyhowResult<R>
    where
        Op: FnOnce(&Page) -> AnyhowResult<R>,
        F: Fn(&AppConfig) -> Result<BrowserTabs, BrowserError>,
    {
        let tabs = self.ensure_tabs(&factory)?;
        let guard = tabs.lock().map_err(|_| BrowserError::LockPoisoned)?;
        let new_page = guard.chromium_page.new_tab(None)?;
        let result = op(&new_page);
        let _ = new_page.close();
        result
    }

    fn ensure_tabs<F>(&self, factory: &F) -> Result<Arc<Mutex<BrowserTabs>>, BrowserError>
    where
        F: Fn(&AppConfig) -> Result<BrowserTabs, BrowserError>,
    {
        let mut guard = self.tabs.lock().map_err(|_| BrowserError::LockPoisoned)?;
        if let Some(tabs) = guard.as_ref() {
            return Ok(tabs.clone());
        }

        let config = self.config.get().ok_or(BrowserError::NotConfigured)?;
        let tabs = Arc::new(Mutex::new(factory(config)?));
        *guard = Some(tabs.clone());
        Ok(tabs)
    }

}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::McpConfig;

    #[test]
    fn returns_not_configured_when_accessed_before_init() {
        let state = BrowserState::new();

        let result = state.with_tab(TabType::Boss, |_| Ok::<_, anyhow::Error>(()), create_browser);

        assert_eq!(
            result.expect_err("missing config should fail").to_string(),
            BrowserError::NotConfigured.to_string()
        );
    }

    #[test]
    fn includes_no_sandbox_flag_in_browser_config() {
        let config = AppConfig {
            browser_exe_path: None,
            user_data_dir: Some("profile-a".to_string()),
            qr_output_path: Some("qr_code.png".to_string()),
            mcp: McpConfig::default(),
        };

        let browser_config = build_browser_config(&config);
        let debug_output = format!("{browser_config:?}");

        assert!(debug_output.contains("--no-sandbox"));
    }

}
