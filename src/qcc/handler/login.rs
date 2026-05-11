use std::{fs, path::PathBuf, time::Duration};

use crate::{browser, qcc::QCC_SITE_URL};
use anyhow::{Context, anyhow};
use base64::Engine;
use rust_drission::utils::sleep_random_ms;
use serde_json::Value;

use crate::config::AppConfig;

pub fn login(config: &AppConfig) -> Result<PathBuf, anyhow::Error> {
    let qr_output_path = config.qr_output_path().to_string();

    let output_path = browser::with_qcc_tab(|page| {
        page.get(QCC_SITE_URL)?;
        sleep_random_ms(500, 800);
        page.click(".qcc-header-login-btn")?;
        sleep_random_ms(500, 800);
        page.wait(".qcc-login-qrcode-area canvas", Duration::from_secs(5))?;
        save_qr_image(page, &qr_output_path)
    })?;
    Ok(output_path)
}

fn save_qr_image(
    page: &rust_drission::Page,
    configured_output_path: &str,
) -> Result<PathBuf, anyhow::Error> {
    let script = r#"
        (() => {
            const canvas = document.querySelector('.qcc-login-qrcode-area canvas');
            if (!canvas) {
                throw new Error('QR canvas not found');
            }

            return canvas.toDataURL('image/png');
        })()
        "#;

    let data_url = page
        .run_js(script)
        .context("浏览器执行二维码导出脚本失败")?;

    let (_, bytes) = decode_data_url(&data_url)?;
    let output_path = qr_output_path(configured_output_path);
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

fn qr_output_path(configured_output_path: &str) -> PathBuf {
    let path = PathBuf::from(configured_output_path);
    let dir = if path.file_name().map_or(false, |n| {
        n.to_string_lossy().contains('.')
    }) {
        path.parent().unwrap_or(&path).to_path_buf()
    } else {
        path
    };
    dir.join("qr_code_qcc.png")
}
