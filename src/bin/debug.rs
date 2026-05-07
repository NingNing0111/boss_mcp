use std::path::Path;

use anyhow::anyhow;
use boss_mcp::{boss::BOSS_LOGIN_PAGE_URL, browser, config::load_or_create};
use rust_drission::utils::sleep_random_ms;

pub fn main() -> Result<(), anyhow::Error> {
    let config = load_or_create(Path::new("config.yaml"))
        .map_err(|e| anyhow!(format!("配置加载异常:{}", e)))?;

    browser::init(config).map_err(|e| anyhow!(format!("浏览器加载异常:{}", e)))?;
    browser::with_browser(|page| {
        page.get(BOSS_LOGIN_PAGE_URL)?;

        Ok(())
    })?;

    sleep_random_ms(100000, 500000);

    Ok(())
}
