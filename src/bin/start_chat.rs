use std::path::Path;

use anyhow::anyhow;
use boss_mcp::{
    boss::handler::{ send_greeting_message, start_chat},
    browser,
    config::load_or_create,
};
use rust_drission::utils::sleep_random_ms;

pub fn main() -> Result<(), anyhow::Error> {
    let config = load_or_create(Path::new("config.yaml"))
        .map_err(|e| anyhow!(format!("配置加载异常:{}", e)))?;

    browser::init(config.clone()).map_err(|e| anyhow!(format!("浏览器加载异常:{}", e)))?;
    let detail_url = "https://www.zhipin.com/job_detail/ca4eec78649de7d60nd82N21FltQ.html";
    start_chat(&detail_url)?;
    sleep_random_ms(1000, 1200);
    browser::with_browser(|page| {
        send_greeting_message(&page, "你好 我对贵公司发布的岗位很感兴趣")?;
        Ok(())
    })?;

    Ok(())
}
