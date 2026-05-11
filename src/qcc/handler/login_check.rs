// 企查查 登录检查
use crate::browser;
use rust_drission::utils::sleep_random_ms;

pub fn login_check() -> Result<String, anyhow::Error> {
    let tmp_url = "https://www.qcc.com/firm/2714d3c87897d572954c3a3360f64632.html";
    let status_str = browser::with_qcc_tab(|page| {
        page.get(tmp_url)?;
        sleep_random_ms(1000, 1200);
        if page.url()?.eq(tmp_url) {
            return Ok("登录状态：成功");
        }
        return Ok("登录状态：未登录");
    })?;

    Ok(status_str.to_string())
}
