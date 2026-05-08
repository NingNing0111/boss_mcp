use std::time::Duration;

use anyhow::Context;
use rust_drission::utils::sleep_random_ms;
use serde::Deserialize;

use crate::{boss::model::PositionDetail, browser};

#[derive(Debug, Deserialize)]
struct RawPositionDetail {
    keywords: Vec<String>,
    job_description: String,
    recruiter_name: String,
    recruiter_title: String,
    recruiter_active_time: String,
    recruiter_company: String,
}

// 获取岗位详情
pub fn position_detail(detail_url: &str) -> Result<PositionDetail, anyhow::Error> {
    let detail = browser::with_browser(|page| {
        page.get(detail_url)?;
        page.wait(".job-detail", Duration::from_secs(5))?;

        let js_result = page.run_js(EXTRACT_JS)?;
        parse_detail(&js_result)
    })?;

    Ok(detail)
}

const EXTRACT_JS: &str = r#"
(() => {
    const normalizeText = (value) => (value || '').replace(/\s+/g, ' ').trim();
    const detail = document.querySelector('.job-detail');
    if (!detail) {
        return JSON.stringify({
            keywords: [],
            job_description: '',
            recruiter_name: '',
            recruiter_title: '',
            recruiter_active_time: '',
            recruiter_company: '',
        });
    }

    const keywords = Array.from(detail.querySelectorAll('.job-keyword-list li'))
        .map((el) => normalizeText(el.textContent))
        .filter(Boolean);

    const jobDescription = (() => {
        const el = detail.querySelector('.job-sec-text');
        return normalizeText(el ? el.textContent : '');
    })();

    const recruiterName = (() => {
        const el = detail.querySelector('.job-boss-info .name');
        return normalizeText(el ? el.childNodes[0]?.textContent || '' : '');
    })();

    const recruiterActiveTime = (() => {
        const el = detail.querySelector('.job-boss-info .boss-active-time');
        return normalizeText(el ? el.textContent : '');
    })();

    const recruiterInfo = (() => {
        const el = detail.querySelector('.job-boss-info .boss-info-attr');
        const text = normalizeText(el ? el.textContent : '');
        const parts = text.split('·').map((item) => item.trim()).filter(Boolean);
        return {
            recruiter_company: parts[0] || '',
            recruiter_title: parts.slice(1).join('·'),
        };
    })();

    return JSON.stringify({
        keywords,
        job_description: jobDescription,
        recruiter_name: recruiterName,
        recruiter_title: recruiterInfo.recruiter_title,
        recruiter_active_time: recruiterActiveTime,
        recruiter_company: recruiterInfo.recruiter_company,
    });
})()
"#;

fn parse_detail(js_result: &serde_json::Value) -> Result<PositionDetail, anyhow::Error> {
    let json_str = js_result
        .as_str()
        .or_else(|| js_result.get("value").and_then(|value| value.as_str()))
        .context("JS 返回值不是字符串")?;

    let raw: RawPositionDetail =
        serde_json::from_str(json_str).context("岗位详情 JSON 解析失败")?;

    Ok(PositionDetail {
        keywords: raw.keywords,
        job_description: raw.job_description,
        recruiter_name: raw.recruiter_name,
        recruiter_title: raw.recruiter_title,
        recruiter_active_time: raw.recruiter_active_time,
        recruiter_company: raw.recruiter_company,
    })
}

// 开始沟通
pub fn start_chat(detail_url: &str) -> Result<(), anyhow::Error> {
    browser::with_browser(|page| {
        page.get(detail_url)?;

        sleep_random_ms(1000, 1200);
        page.click(".btn-container .btn-startchat")?;

        Ok(())
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn parses_position_detail_from_js_string_result() {
        let js_result = json!(
            r#"{"keywords":["rust","来自BOSS直聘流量处理"],"job_description":"熟练掌握Rust语言的后端高级开发工程师","recruiter_name":"徐盟","recruiter_title":"HRBP主管","recruiter_active_time":"刚刚活跃","recruiter_company":"灵云数科"}"#
        );

        let detail = parse_detail(&js_result).expect("should parse position detail");

        assert_eq!(
            detail,
            PositionDetail {
                keywords: vec!["rust".to_string(), "来自BOSS直聘流量处理".to_string()],
                job_description: "熟练掌握Rust语言的后端高级开发工程师".to_string(),
                recruiter_name: "徐盟".to_string(),
                recruiter_title: "HRBP主管".to_string(),
                recruiter_active_time: "刚刚活跃".to_string(),
                recruiter_company: "灵云数科".to_string(),
            }
        );
    }

    #[test]
    fn returns_error_when_js_result_is_not_a_string() {
        let js_result = json!({"value": 123});

        let error = parse_detail(&js_result).expect_err("non-string result should fail");

        assert!(error.to_string().contains("JS 返回值不是字符串"));
    }

    #[test]
    fn returns_error_when_json_shape_is_invalid() {
        let js_result = json!(r#"{"keywords":"rust"}"#);

        let error = parse_detail(&js_result).expect_err("invalid json shape should fail");

        assert!(error.to_string().contains("岗位详情 JSON 解析失败"));
    }
}
