use std::path::Path;

use anyhow::{Context, anyhow};
use boss_mcp::{
    boss::{BOSS_ACCOUNT_VERIFY_API, BOSS_LOGIN_PAGE_URL},
    browser,
    config::load_or_create,
};
use rust_drission::{ChromiumPage, utils::sleep_random_ms};
use serde_json::json;

pub fn main() -> Result<(), anyhow::Error> {
    let config = load_or_create(Path::new("config.yaml"))
        .map_err(|e| anyhow!(format!("配置加载异常:{}", e)))?;

    browser::init(config).map_err(|e| anyhow!(format!("浏览器加载异常:{}", e)))?;

    let verify_result = browser::with_browser(|page| verify_login(&*page));
    let output = build_login_check_output(
        verify_result.map_err(|e| anyhow!("登录状态异常:{}", summarize_error(&e))),
    );

    println!("{}", serde_json::to_string(&output)?);

    Ok(())
}

fn verify_login(page: &ChromiumPage) -> Result<(), anyhow::Error> {
    page.get(BOSS_LOGIN_PAGE_URL)?;
    sleep_random_ms(1200, 2000);
    let body_text = fetch_via_page_js(page, BOSS_ACCOUNT_VERIFY_API)?;
    println!("{:?}", body_text);
    if body_text.starts_with("ERR:") {
        return Err(anyhow!("token校验请求失败: {}", body_text));
    }

    parse_verify_response(&body_text)
}

fn fetch_via_page_js(page: &ChromiumPage, url: &str) -> Result<String, anyhow::Error> {
    let script = build_fetch_script(url);
    let result = page.run_js_await(&script)?;
    result
        .get("value")
        .and_then(serde_json::Value::as_str)
        .map(|s| s.to_string())
        .context("页面 fetch 返回值非字符串（缺少 result.value）")
}

fn build_fetch_script(url: &str) -> String {
    format!(
        r#"
        (async () => {{
            try {{
                const response = await fetch({:?}, {{
                    method: 'GET',
                    credentials: 'include'
                }});
                const text = await response.text();
                if (!response.ok) {{
                    return "ERR: HTTP " + response.status + " " + response.statusText + " body=" + text;
                }}
                return text;
            }} catch (e) {{
                return "ERR: " + (e.message || String(e));
            }}
        }})()
        "#,
        url
    )
}

fn parse_verify_response(body_text: &str) -> Result<(), anyhow::Error> {
    let root: serde_json::Value =
        serde_json::from_str(body_text).context("解析 token 校验 JSON 失败")?;

    let code = root.get("code").and_then(|v| v.as_i64()).unwrap_or(-1);
    if code != 0 {
        return Err(anyhow!(
            "接口业务失败: code={}, message={}",
            code,
            root.get("message")
                .and_then(|v| v.as_str())
                .unwrap_or("unknown")
        ));
    }

    match root.get("zpData").and_then(|v| v.as_bool()) {
        Some(true) => Ok(()),
        Some(false) => Err(anyhow!("token校验失败")),
        None => Err(anyhow!("响应缺少布尔类型 zpData")),
    }
}

fn build_login_check_output(verify_result: Result<(), anyhow::Error>) -> serde_json::Value {
    match verify_result {
        Ok(()) => json!({
            "success": true,
            "message": "登录成功",
        }),
        Err(error) => json!({
            "success": false,
            "message": "登录校验异常",
            "error": summarize_error(&error),
        }),
    }
}

fn summarize_error(error: &anyhow::Error) -> String {
    let message = error.to_string();

    message
        .rsplit(": ")
        .next()
        .map(str::to_string)
        .unwrap_or(message)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builds_post_fetch_script_for_token_verify() {
        let script = build_fetch_script("https://example.com/verify");

        assert!(script.contains("method: 'POST'"));
        assert!(script.contains("credentials: 'include'"));
    }

    #[test]
    fn returns_success_when_token_verify_result_is_true() {
        let output = build_login_check_output(Ok(()));

        assert_eq!(output["success"], true);
        assert_eq!(output["message"], "登录成功");
        assert!(output.get("data").is_none());
    }

    #[test]
    fn returns_brief_error_when_token_verify_fails() {
        let output =
            build_login_check_output(Err(anyhow!("登录状态异常: token verify request timeout")));

        assert_eq!(output["success"], false);
        assert_eq!(output["message"], "登录校验异常");
        assert_eq!(output["error"], "token verify request timeout");
    }

    #[test]
    fn accepts_true_zpdata_from_verify_api() {
        let result = parse_verify_response(r#"{"code":0,"message":"Success","zpData":true}"#);

        assert!(result.is_ok());
    }

    #[test]
    fn rejects_false_zpdata_from_verify_api() {
        let error = parse_verify_response(r#"{"code":0,"message":"Success","zpData":false}"#)
            .expect_err("false zpData should fail");

        assert_eq!(error.to_string(), "token校验失败");
    }
}
