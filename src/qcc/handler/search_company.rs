use anyhow::Context;
use rust_drission::utils::sleep_random_ms;
use serde::Deserialize;

use crate::qcc::QCC_SEARCH;
use crate::{browser, qcc::model::CompanyInfo};

pub fn search_company(keyword: &str) -> Result<Vec<CompanyInfo>, anyhow::Error> {
    let url = QCC_SEARCH.replace("{}", keyword);

    let companies = browser::with_browser(|page| {
        page.get(&url)?;
        sleep_random_ms(2000, 3000);

        let js_result = page.run_js(EXTRACT_JS)?;
        parse_companies(&js_result)
    })?;

    Ok(companies)
}

#[derive(Debug, Deserialize)]
struct RawCompany {
    name: String,
    tags: String,
    status: String,
    legal_person: String,
    registered_capital: String,
    detail_url: String,
    established_date: String,
    shareholder: String,
}

const EXTRACT_JS: &str = r#"
(() => {
    const rows = document.querySelectorAll('table.app-ltable tr');
    const results = [];

    for (const row of rows) {
        const maininfo = row.querySelector('.maininfo');
        if (!maininfo) continue;

        const titleEl = maininfo.querySelector('.copy-title a.title');
        const statusEl = maininfo.querySelector('.nstatus');
        const relateInfo = maininfo.querySelector('.relate-info');
        if (!titleEl || !relateInfo) continue;

        const name = titleEl.textContent.trim();
        const detailUrl = titleEl.href || '';
        const status = statusEl ? statusEl.textContent.trim() : '';

        const tags = (() => {
            const tagEls = maininfo.querySelectorAll('.tags-list .ntag span span');
            return Array.from(tagEls).map(el => el.textContent.trim()).filter(Boolean).join(', ');
        })();

        const legalPerson = (() => {
            const el = relateInfo.querySelector('.app-coy-is-person');
            return el ? el.textContent.trim() : '';
        })();

        const registeredCapital = (() => {
            const spans = relateInfo.querySelectorAll('.rline .f');
            for (const s of spans) {
                if (s.textContent.includes('注册资本')) {
                    const val = s.querySelector('.val');
                    return val ? val.textContent.trim() : '';
                }
            }
            return '';
        })();

        const establishedDate = (() => {
            const spans = relateInfo.querySelectorAll('.rline .f');
            for (const s of spans) {
                if (s.textContent.includes('成立日期')) {
                    const val = s.querySelector('.val');
                    return val ? val.textContent.trim() : '';
                }
            }
            return '';
        })();

        const shareholder = (() => {
            const el = relateInfo.querySelector('.hit-reasons-list .sf');
            if (el && el.textContent.includes('股东')) {
                const a = el.querySelector('a.text-primary');
                return a ? a.textContent.trim() : '';
            }
            return '';
        })();

        results.push({
            name,
            tags,
            status,
            legal_person: legalPerson,
            registered_capital: registeredCapital,
            detail_url: detailUrl,
            established_date: establishedDate,
            shareholder,
        });
    }

    return JSON.stringify(results);
})()
"#;

fn parse_companies(js_result: &serde_json::Value) -> Result<Vec<CompanyInfo>, anyhow::Error> {
    let json_str = js_result
        .as_str()
        .or_else(|| js_result.get("value").and_then(|v| v.as_str()))
        .context("JS 返回值不是字符串")?;

    let raw: Vec<RawCompany> = serde_json::from_str(json_str).context("公司信息 JSON 解析失败")?;

    Ok(raw
        .into_iter()
        .map(|r| CompanyInfo {
            name: r.name,
            tags: r.tags,
            status: r.status,
            legal_person: r.legal_person,
            registered_capital: r.registered_capital,
            detail_url: r.detail_url,
            established_date: r.established_date,
            shareholder: r.shareholder,
        })
        .collect())
}
