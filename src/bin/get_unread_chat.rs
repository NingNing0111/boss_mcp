use std::path::Path;

use anyhow::anyhow;
use boss_mcp::{boss::handler::{get_unread_chat, get_unread_chat_message}, browser, config::load_or_create};

pub fn main() -> Result<(), anyhow::Error> {
    let config = load_or_create(Path::new("config.yaml"))
        .map_err(|e| anyhow!(format!("配置加载异常:{}", e)))?;

    browser::init(config).map_err(|e| anyhow!(format!("浏览器加载异常:{}", e)))?;
    let output = get_unread_chat()?;
    println!("{:?}", output);
    let chat_messages = get_unread_chat_message(1)?;
    println!("{:?}", chat_messages);
    Ok(())
}


