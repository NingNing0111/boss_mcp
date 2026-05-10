use std::{fs, path::PathBuf, time::Duration};

use crate::{
    boss::{BOSS_LOGIN_PAGE_URL, model::LoginType},
    browser,
    config::AppConfig,
};
use anyhow::{Context, Ok, anyhow};
use base64::Engine;
use rust_drission::utils::sleep_random_ms;
use serde_json::Value;

// 验证码登录/注册
// BOSS直聘APP扫码登录
pub fn login(tp: LoginType, config: &AppConfig) -> Result<PathBuf, anyhow::Error> {
    let output_path = browser::with_browser(|page| {
        page.get(BOSS_LOGIN_PAGE_URL)?;
        sleep_random_ms(800, 1000);
        let title_ele = page.ele(".title")?;
        let text = title_ele.unwrap().text_content()?;
        if tp == LoginType::QRCode && text.contains("验证码登录") {
            page.wait(".ewm-switch", Duration::from_secs(5))?;
            page.click(".ewm-switch")?;
            println!("点击了QR");
        }
        sleep_random_ms(800, 1000);

        page.wait(".qr-img-box", Duration::from_secs(5))?;
        save_qr_image(page, config.qr_output_path())
    })?;
    Ok(output_path)
}

fn save_qr_image(
    page: &rust_drission::ChromiumPage,
    configured_output_path: &str,
) -> Result<PathBuf, anyhow::Error> {
    let img = page
        .ele(".qr-img-box img")?
        .ok_or_else(|| anyhow!("未找到二维码图片元素"))?;

    let src = img.attr("src")?;

    let script = format!(
        r#"
        (async () => {{
            const img = document.querySelector('.qr-img-box img');
            if (!img) {{
                throw new Error('QR image not found');
            }}

            const src = img.getAttribute('src');
            if (!src) {{
                throw new Error('QR image src missing');
            }}

            const response = await fetch({src:?});
            const blob = await response.blob();

            return await new Promise((resolve, reject) => {{
                const reader = new FileReader();
                reader.onloadend = () => resolve(reader.result);
                reader.onerror = () => reject(new Error('failed to read blob as data url'));
                reader.readAsDataURL(blob);
            }});
        }})()
        "#
    );

    let data_url = page
        .run_js_await(&script)
        .context("浏览器执行二维码导出脚本失败")?;

    let (content_type, bytes) = decode_data_url(&data_url)?;
    let output_path = qr_output_path(configured_output_path, content_type);
    if let Some(parent) = output_path
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
    {
        fs::create_dir_all(parent)
            .with_context(|| format!("二维码目录创建失败: {}", parent.display()))?;
    }
    fs::write(&output_path, bytes)
        .with_context(|| format!("二维码保存失败: {}", output_path.display()))?;
    Ok(output_path)
}

fn decode_data_url(value: &Value) -> Result<(&str, Vec<u8>), anyhow::Error> {
    let data_url = extract_js_string(value)?;
    let prefix = "data:";
    let base64_marker = ";base64,";

    let body = data_url
        .strip_prefix(prefix)
        .ok_or_else(|| anyhow!("二维码导出结果不是 data URL"))?;
    let (content_type, encoded) = body
        .split_once(base64_marker)
        .ok_or_else(|| anyhow!("二维码导出结果不是 base64 data URL"))?;

    let bytes = base64::engine::general_purpose::STANDARD
        .decode(encoded)
        .context("二维码 base64 解码失败")?;

    Ok((content_type, bytes))
}

fn extract_js_string(value: &Value) -> Result<&str, anyhow::Error> {
    if let Some(text) = value.as_str() {
        return Ok(text);
    }

    if let Some(text) = value.get("value").and_then(Value::as_str) {
        return Ok(text);
    }

    if value.get("subtype").and_then(Value::as_str) == Some("error") {
        let message = value
            .get("description")
            .and_then(Value::as_str)
            .unwrap_or("浏览器脚本执行失败");
        return Err(anyhow!("浏览器脚本执行失败: {message}"));
    }

    value
        .get("description")
        .and_then(Value::as_str)
        .ok_or_else(|| anyhow!("二维码导出结果不是字符串"))
}

fn qr_output_path(configured_output_path: &str, content_type: &str) -> std::path::PathBuf {
    let extension = match content_type {
        "image/png" => "png",
        "image/jpeg" => "jpg",
        "image/webp" => "webp",
        "image/gif" => "gif",
        _ => "bin",
    };

    PathBuf::from(configured_output_path).with_extension(extension)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn keeps_png_extension_for_png_content() {
        let path = qr_output_path("/data/qr/qr_code.png", "image/png");
        assert_eq!(path, PathBuf::from("/data/qr/qr_code.png"));
    }

    #[test]
    fn rewrites_extension_for_jpeg_content() {
        let path = qr_output_path("/data/qr/qr_code.png", "image/jpeg");
        assert_eq!(path, PathBuf::from("/data/qr/qr_code.jpg"));
    }
}
