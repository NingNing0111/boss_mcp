use std::time::Duration;

use anyhow::Context;

use crate::{boss::model::PositionSimpleInfo, browser, utils::salary::decode_salary};

pub fn search_position(search_url: &str) -> Result<Vec<PositionSimpleInfo>, anyhow::Error> {
    let positions = browser::with_boss_tab(|page| {
        page.get(search_url)?;
        page.wait(".rec-job-list", Duration::from_secs(5))?;
        let js_result = page.run_js(EXTRACT_JS)?;
        parse_positions(&js_result)
    })?;

    Ok(positions)
}

const EXTRACT_JS: &str = r#"
(() => {
    const normalizeText = (value) => value.replace(/\s+/g, ' ').trim();
    const cards = document.querySelectorAll('.job-card-box');
    const results = [];

    for (const card of cards) {
        const jobNameEl = card.querySelector('.job-name');
        const salaryEl = card.querySelector('.job-salary');
        const companyNameEl = card.querySelector('.boss-name');
        const companyLocationEl = card.querySelector('.company-location');
        const companyLinkEl = card.querySelector('.boss-info');

        if (!jobNameEl || !salaryEl || !companyNameEl || !companyLocationEl) {
            continue;
        }

        const companyUrl = companyLinkEl ? companyLinkEl.href || '' : '';
        const tags = Array.from(card.querySelectorAll('.tag-list li'))
            .map((el) => normalizeText(el.textContent))
            .filter(Boolean);

        results.push({
            job_name: normalizeText(jobNameEl.textContent),
            salary: normalizeText(salaryEl.textContent),
            tags,
            company_name: normalizeText(companyNameEl.textContent),
            company_location: normalizeText(companyLocationEl.textContent),
            job_detail_url: jobNameEl.href || '',
            company_url: companyUrl.startsWith('javascript:') ? '' : companyUrl,
        });
    }

    return JSON.stringify(results);
})()
"#;

fn parse_positions(
    js_result: &serde_json::Value,
) -> Result<Vec<PositionSimpleInfo>, anyhow::Error> {
    let json_str = extract_json_str(js_result)?;
    let positions: Vec<PositionSimpleInfo> =
        serde_json::from_str(json_str).context("岗位信息 JSON 解析失败")?;

    Ok(positions
        .into_iter()
        .map(|position| PositionSimpleInfo {
            salary: decode_salary(&position.salary),
            ..position
        })
        .collect())
}

fn extract_json_str(js_result: &serde_json::Value) -> Result<&str, anyhow::Error> {
    js_result
        .as_str()
        .or_else(|| js_result.get("value").and_then(|value| value.as_str()))
        .context("JS 返回值不是字符串")
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn parses_position_simple_info_from_js_string_result() {
        let js_result = json!(
            r#"[{"job_name":"AI 全栈开发工程师（Electron+Vue+Rust）","salary":"-K","tags":["3-5年","本科"],"company_name":"上海晟之福科技","company_location":"北京·朝阳区·亚运村","job_detail_url":"/job_detail/a9428ad52c22b3750ndy3dS7FlVS.html","company_url":"/gongsi/5a9c0f85075f282a03By0964GVY~.html?from=top-card"}]"#
        );

        let positions = parse_positions(&js_result).expect("should parse positions");

        assert_eq!(positions.len(), 1);
        assert_eq!(
            positions[0].job_name,
            "AI 全栈开发工程师（Electron+Vue+Rust）"
        );
        assert_eq!(positions[0].salary, "9-11K");
        assert_eq!(positions[0].tags, vec!["3-5年", "本科"]);
        assert_eq!(positions[0].company_name, "上海晟之福科技");
        assert_eq!(positions[0].company_location, "北京·朝阳区·亚运村");
        assert_eq!(
            positions[0].job_detail_url,
            "/job_detail/a9428ad52c22b3750ndy3dS7FlVS.html"
        );
        assert_eq!(
            positions[0].company_url,
            "/gongsi/5a9c0f85075f282a03By0964GVY~.html?from=top-card"
        );
    }

    #[test]
    fn parses_position_simple_info_from_wrapped_js_result() {
        let js_result = json!({
            "value": r#"[{"job_name":"Rust专家（remote）","salary":"-K·薪","tags":["5-10年","本科"],"company_name":"某知名企业","company_location":"北京","job_detail_url":"/job_detail/d2e24532c08488a40nV53tW5GVRY.html","company_url":""}]"#
        });

        let positions = parse_positions(&js_result).expect("should parse wrapped positions");

        assert_eq!(positions.len(), 1);
        assert_eq!(positions[0].job_name, "Rust专家（remote）");
        assert_eq!(positions[0].company_name, "某知名企业");
        assert_eq!(positions[0].company_location, "北京");
        assert_eq!(positions[0].company_url, "");
    }
}
