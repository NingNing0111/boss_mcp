use std::fmt;
use std::sync::{Mutex, OnceLock};

use anyhow::Result as AnyhowResult;
use rust_drission::{BrowserConfig, ChromiumPage};

use crate::config::AppConfig;

static BROWSER_STATE: BrowserState<ChromiumPage> = BrowserState::new();

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

pub fn init(config: AppConfig) -> Result<(), BrowserError> {
    BROWSER_STATE.init(config, create_browser)
}

pub fn with_browser<T, F>(op: F) -> AnyhowResult<T>
where
    F: FnOnce(&mut ChromiumPage) -> AnyhowResult<T>,
{
    BROWSER_STATE.with_browser(op, create_browser)
}

fn create_browser(config: &AppConfig) -> Result<ChromiumPage, BrowserError> {
    let browser_config = build_browser_config(config);

    ChromiumPage::new(browser_config).map_err(|err| BrowserError::InitFailed(err.to_string()))
}

fn build_browser_config(config: &AppConfig) -> BrowserConfig {
    let user_data = config.user_data_dir();
    println!("{}", user_data);
    let mut browser_config = BrowserConfig::new()
        .headless(false)
        .set_argument("--no-sandbox", None::<&str>)
        .user_data_dir(user_data);

    if let Some(path) = config.browser_exe_path() {
        browser_config = browser_config.chrome_path(path);
    }

    browser_config
}

struct BrowserState<T> {
    config: OnceLock<AppConfig>,
    page: OnceLock<Mutex<T>>,
}

impl<T> BrowserState<T> {
    const fn new() -> Self {
        Self {
            config: OnceLock::new(),
            page: OnceLock::new(),
        }
    }

    fn init<F>(&self, config: AppConfig, factory: F) -> Result<(), BrowserError>
    where
        F: Fn(&AppConfig) -> Result<T, BrowserError>,
    {
        let _ = self.config.set(config);
        let _ = self.ensure_page(&factory)?;
        Ok(())
    }

    fn with_browser<R, Op, F>(&self, op: Op, factory: F) -> AnyhowResult<R>
    where
        Op: FnOnce(&mut T) -> AnyhowResult<R>,
        F: Fn(&AppConfig) -> Result<T, BrowserError>,
    {
        let page = self.ensure_page(&factory)?;
        let mut guard = page.lock().map_err(|_| BrowserError::LockPoisoned)?;
        op(&mut guard)
    }

    fn ensure_page<F>(&self, factory: &F) -> Result<&Mutex<T>, BrowserError>
    where
        F: Fn(&AppConfig) -> Result<T, BrowserError>,
    {
        if let Some(page) = self.page.get() {
            return Ok(page);
        }

        let config = self.config.get().ok_or(BrowserError::NotConfigured)?;
        let page = Mutex::new(factory(config)?);

        match self.page.set(page) {
            Ok(()) => self.page.get().ok_or(BrowserError::NotConfigured),
            Err(_) => self.page.get().ok_or(BrowserError::NotConfigured),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::McpConfig;
    use std::sync::Arc;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::thread;

    #[derive(Debug)]
    struct FakeBrowser {
        config_name: String,
        visits: usize,
    }

    #[test]
    fn returns_not_configured_when_accessed_before_init() {
        let state = BrowserState::<FakeBrowser>::new();

        let result = state.with_browser(|_| Ok::<(), anyhow::Error>(()), fake_factory);

        assert_eq!(
            result.expect_err("missing config should fail").to_string(),
            BrowserError::NotConfigured.to_string()
        );
    }

    #[test]
    fn uses_saved_config_for_lazy_initialization() {
        let state = BrowserState::<FakeBrowser>::new();
        let config = sample_config("first", "profile-a");
        let factory_calls = Arc::new(AtomicUsize::new(0));

        state.config.set(config).expect("config should be set once");

        let result = state.with_browser(
            |browser| {
                browser.visits += 1;
                Ok::<_, anyhow::Error>((browser.config_name.clone(), browser.visits))
            },
            counting_fake_factory(factory_calls.clone()),
        );

        let value = result.expect("lazy init should succeed");
        assert_eq!(value, ("first".to_string(), 1));
        assert_eq!(factory_calls.load(Ordering::SeqCst), 1);
    }

    #[test]
    fn repeated_init_keeps_first_config_and_singleton() {
        let state = BrowserState::<FakeBrowser>::new();
        let factory_calls = Arc::new(AtomicUsize::new(0));

        state
            .init(
                sample_config("first", "profile-a"),
                counting_fake_factory(factory_calls.clone()),
            )
            .expect("first init should succeed");
        state
            .init(
                sample_config("second", "profile-b"),
                counting_fake_factory(factory_calls.clone()),
            )
            .expect("second init should be ignored");

        let result = state.with_browser(
            |browser| Ok::<_, anyhow::Error>(browser.config_name.clone()),
            counting_fake_factory(factory_calls.clone()),
        );

        let value = result.expect("singleton should keep first config");
        assert_eq!(value, "first".to_string());
        assert_eq!(factory_calls.load(Ordering::SeqCst), 1);
    }

    #[test]
    fn allows_anyhow_errors_from_browser_operations() {
        let state = BrowserState::<FakeBrowser>::new();
        let config = sample_config("shared", "profile-a");

        state.config.set(config).expect("config should be set once");

        let result: AnyhowResult<()> =
            state.with_browser(|_| Err(anyhow::anyhow!("operation failed")), fake_factory);

        assert_eq!(
            result
                .expect_err("closure error should bubble up")
                .to_string(),
            "operation failed"
        );
    }

    #[test]
    fn concurrent_access_reuses_single_instance() {
        let state = Arc::new(BrowserState::<FakeBrowser>::new());
        let factory_calls = Arc::new(AtomicUsize::new(0));

        state
            .config
            .set(sample_config("shared", "profile-a"))
            .expect("config should be set once");

        let handles: Vec<_> = (0..4)
            .map(|_| {
                let state = state.clone();
                let factory_calls = factory_calls.clone();
                thread::spawn(move || {
                    state.with_browser(
                        |browser| {
                            browser.visits += 1;
                            Ok::<_, anyhow::Error>(browser.config_name.clone())
                        },
                        counting_fake_factory(factory_calls),
                    )
                })
            })
            .collect();

        let results: Vec<_> = handles
            .into_iter()
            .map(|handle| handle.join().expect("thread should finish"))
            .collect();

        assert!(
            results
                .iter()
                .all(|result| matches!(result, Ok(value) if value == "shared"))
        );
        assert_eq!(factory_calls.load(Ordering::SeqCst), 1);

        let final_visits = state
            .with_browser(
                |browser| Ok::<_, anyhow::Error>(browser.visits),
                fake_factory,
            )
            .expect("final read should succeed");
        assert_eq!(final_visits, 4);
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

    fn fake_factory(config: &AppConfig) -> Result<FakeBrowser, BrowserError> {
        Ok(FakeBrowser {
            config_name: config.browser_exe_path().unwrap_or("missing").to_string(),
            visits: 0,
        })
    }

    fn counting_fake_factory(
        factory_calls: Arc<AtomicUsize>,
    ) -> impl Fn(&AppConfig) -> Result<FakeBrowser, BrowserError> {
        move |config| {
            factory_calls.fetch_add(1, Ordering::SeqCst);
            fake_factory(config)
        }
    }

    fn sample_config(browser_exe_path: &str, user_data_dir: &str) -> AppConfig {
        AppConfig {
            browser_exe_path: Some(browser_exe_path.to_string()),
            user_data_dir: Some(user_data_dir.to_string()),
            qr_output_path: Some("qr_code.png".to_string()),
            mcp: McpConfig::default(),
        }
    }
}
