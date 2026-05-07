use std::path::Path;

use anyhow::anyhow;
use boss_mcp::{browser, config::load_or_create};
use rust_drission::utils::sleep_random_ms;

fn main() -> Result<(), anyhow::Error> {
    let config = load_or_create(Path::new("config.yaml"))
        .map_err(|e| anyhow!(format!("配置加载异常:{}", e)))?;

    println!("{:?}", config);

    browser::init(config).map_err(|e| anyhow!(format!("浏览器加载异常:{}", e)))?;

    browser::with_browser(|page| {
        page.get("https://www.zhipin.com")?;

        Ok(())
    })
    .map_err(|e| anyhow!(format!("{}", e)))?;

    sleep_random_ms(60000, 70000);
    Ok(())
}
