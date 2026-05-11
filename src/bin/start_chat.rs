use std::path::Path;

use anyhow::anyhow;
use boss_mcp::{
    boss::handler::{ start_chat},
    browser,
    config::load_or_create,
};

pub fn main() -> Result<(), anyhow::Error> {
    let config = load_or_create(Path::new("config.yaml"))
        .map_err(|e| anyhow!(format!("配置加载异常:{}", e)))?;

    browser::init(config.clone()).map_err(|e| anyhow!(format!("浏览器加载异常:{}", e)))?;
    let detail_url = "https://www.zhipin.com/job_detail/128f163c4101c32c0nd82di0GFRQ.html";
    browser::with_boss_tab(|page| {
        page.get(detail_url)?;

        Ok(())
    })?;
    // start_chat(&detail_url, "你好")?;

    Ok(())
}
