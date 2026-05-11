use std::{path::Path, time::Duration};

use anyhow::{Context, anyhow};
use boss_mcp::{
    boss::model::SalaryDebugInfo,
    browser,
    config::load_or_create,
    utils::{
        job_query::{JobSearchParams, build_job_search_url},
        salary::decode_salary,
        site,
    },
};

pub fn main() -> Result<(), anyhow::Error> {
    let config = load_or_create(Path::new("config.yaml"))
        .map_err(|e| anyhow!(format!("配置加载异常:{}", e)))?;

    browser::init(config.clone()).map_err(|e| anyhow!(format!("浏览器加载异常:{}", e)))?;

    let search_params = JobSearchParams {
        city: site::get_city_code_by_name("北京"),
        query: Some("Rust".to_string()),
        ..JobSearchParams::default()
    };
    let search_url = build_job_search_url(&search_params);

    let salary_info = debug_salary_font(&search_url)?;
    let decoded_text = decode_salary(&salary_info.text_content);
    let decoded_inner_text = decode_salary(&salary_info.inner_text);

    println!("{}", salary_info.format_output());
    println!("decodedTextContent: {:?}", decoded_text);
    println!("decodedInnerText: {:?}", decoded_inner_text);

    Ok(())
}

const SALARY_DEBUG_JS: &str = r#"
(() => {
    const el = document.querySelectorAll('.job-salary')[0];
    if (!el) {
        throw new Error('未找到 .job-salary 节点');
    }

    return JSON.stringify({
        text_content: el.textContent || '',
        inner_text: el.innerText || '',
        font_family: getComputedStyle(el).fontFamily || '',
        html: el.outerHTML || '',
    });
})()
"#;

fn debug_salary_font(search_url: &str) -> Result<SalaryDebugInfo, anyhow::Error> {
    let salary_info = browser::with_boss_tab(|page| {
        page.get(search_url)?;
        page.wait(".job-salary", Duration::from_secs(5))?;
        let js_result = page.run_js(SALARY_DEBUG_JS)?;
        parse_salary_debug_info(&js_result)
    })?;

    Ok(salary_info)
}

fn parse_salary_debug_info(
    js_result: &serde_json::Value,
) -> Result<SalaryDebugInfo, anyhow::Error> {
    let json_str = js_result
        .as_str()
        .or_else(|| js_result.get("value").and_then(|value| value.as_str()))
        .context("JS 返回值不是字符串")?;

    serde_json::from_str(json_str).context("薪资调试信息 JSON 解析失败")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn decodes_boss_salary_text_from_known_font_mapping() {
        let encoded_salary = "\u{e038}\u{e032}-\u{e035}\u{e039}K·13薪";

        let decoded = decode_salary(encoded_salary);

        assert_eq!(decoded, "71-48K·13薪");
    }
}
