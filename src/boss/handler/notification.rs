use std::time::Duration;

use rust_drission::utils::sleep_random_ms;

use crate::{boss::BOSS_SITE_URL, browser};

// 消息通知监听
pub fn get_new_count() -> Result<i32, anyhow::Error> {
    let chat_num = browser::with_browser(|page| {
        page.get(BOSS_SITE_URL)?;
        page.wait(".nav-chat-num", Duration::from_secs(15))?;

        for _ in 0..30 {
            let ele = page.ele(".nav-chat-num")?;
            if let Some(ele) = ele {
                let text = ele.text_content()?;

                if !text.trim().is_empty() {
                    let count = text.trim().parse::<i32>().unwrap_or(0);
                    return Ok(count);
                }
            }

            sleep_random_ms(500, 800);
        }

        Ok(1)
    })?;

    Ok(chat_num)
}
