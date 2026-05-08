use std::path::Path;

use anyhow::anyhow;
use boss_mcp::{boss::handler::position_detail, browser, config::load_or_create};

pub fn main() -> Result<(), anyhow::Error> {
    let config = load_or_create(Path::new("config.yaml"))
        .map_err(|e| anyhow!(format!("配置加载异常:{}", e)))?;

    browser::init(config.clone()).map_err(|e| anyhow!(format!("浏览器加载异常:{}", e)))?;
    let detail_url = "https://www.zhipin.com/job_detail/e6773a7b7d79e3310nd_2ti5FFVY.html";
    let result = position_detail(&detail_url)?;

    println!("{}", serde_json::to_string_pretty(&result)?);

    Ok(())
}
