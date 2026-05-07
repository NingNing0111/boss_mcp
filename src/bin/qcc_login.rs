use std::path::Path;

use anyhow::anyhow;
use boss_mcp::{
    browser,
    config::load_or_create,
    qcc::{self},
};

fn main() -> Result<(), anyhow::Error> {
    let config = load_or_create(Path::new("config.yaml"))
        .map_err(|e| anyhow!(format!("配置加载异常:{}", e)))?;
    browser::init(config.clone()).map_err(|e| anyhow!(format!("浏览器加载异常:{}", e)))?;
    let path = qcc::handler::login(&config)?;
    println!("QR Path: {:?}", path);
    Ok(())
}
