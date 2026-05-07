use std::path::Path;

use anyhow::{ anyhow};
use boss_mcp::{
    boss::{ handler::login_check},
    browser,
    config::load_or_create,
};

pub fn main() -> Result<(), anyhow::Error> {
    let config = load_or_create(Path::new("config.yaml"))
        .map_err(|e| anyhow!(format!("配置加载异常:{}", e)))?;

    browser::init(config).map_err(|e| anyhow!(format!("浏览器加载异常:{}", e)))?;
    let output = login_check()?;

    println!("{}", serde_json::to_string(&output)?);

    Ok(())
}

