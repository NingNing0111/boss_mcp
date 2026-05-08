use anyhow::anyhow;
use rust_drission::{ChromiumPage, utils::sleep_random_ms};

// 发送消息
// 发送打招呼 发送则为true 没有发送成功或者已经发过了 则为false
pub fn send_greeting_message(page: &ChromiumPage, greeting: &str) -> Result<bool, anyhow::Error> {

    let greeting_js = serde_json::to_string(greeting).map_err(|e| anyhow!("{}",e))?;
    page.run_js(&format!(
        "document.querySelector('#chat-input').textContent = {};",
        greeting_js
    ))?;
    sleep_random_ms(900, 1500);
    let send_btn_selector = ".chat-op .btn-send";
    let send_btn_ele = page.ele(send_btn_selector)?;
    if let Some(send_btn_ele) = send_btn_ele {
        send_btn_ele.click()?;
        return Ok(true);
    }
    Ok(false)
}
