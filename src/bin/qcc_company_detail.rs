use std::path::Path;

use anyhow::anyhow;
use boss_mcp::{browser, config::load_or_create, qcc::handler::company_detail};

pub fn main() -> Result<(), anyhow::Error> {
    let config = load_or_create(Path::new("config.yaml"))
        .map_err(|e| anyhow!(format!("配置加载异常:{}", e)))?;

    browser::init(config.clone()).map_err(|e| anyhow!(format!("浏览器加载异常:{}", e)))?;
    let detail = "https://www.qcc.com/firm/3b5e1f803226c7eed43c433d9305048e.html";
    let detail = company_detail(detail)?;

    println!("{:?}", detail);

    Ok(())
}
