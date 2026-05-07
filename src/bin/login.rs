use std::path::Path;

use anyhow::anyhow;
use boss_mcp::{
    boss::{handler::login, model::LoginType},
    browser,
    config::load_or_create,
};

pub fn main() -> Result<(), anyhow::Error> {
    let config = load_or_create(Path::new("config.yaml"))
        .map_err(|e| anyhow!(format!("配置加载异常:{}", e)))?;

    browser::init(config.clone()).map_err(|e| anyhow!(format!("浏览器加载异常:{}", e)))?;

    let login_path = login(LoginType::QRCode, &config)?;
    println!("登录二维码所在路径:{}", login_path.display());
    Ok(())
}
