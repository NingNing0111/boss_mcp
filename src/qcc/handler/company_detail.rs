use anyhow::Context;
use rust_drission::utils::sleep_random_ms;
use serde::Deserialize;

use crate::{browser, qcc::model::CompanyDetail};

pub fn company_detail(detail_url: &str) -> Result<CompanyDetail, anyhow::Error> {
    let detail = browser::with_browser(|page| {
        page.get(detail_url)?;
        sleep_random_ms(2000, 3000);

        let js_result = page.run_js(EXTRACT_JS)?;
        parse_detail(&js_result)
    })?;

    Ok(detail)
}

#[derive(Debug, Deserialize)]
struct RawDetail {
    name: String,
    status: String,
    description: String,
    industry: String,
    scale: String,
    employee_count: String,
    insurance_count: String,
    business_scope: String,
    established_date: String,
    registered_capital: String,
    legal_person: String,
    financing_stage: String,
    phone: String,
    website: String,
    address: String,
}

const EXTRACT_JS: &str = r#"
(() => {
    const header = document.querySelector('.company-header');
    if (!header) return JSON.stringify({error: 'company header not found'});

    const name = (() => {
        const el = header.querySelector('.title .copy-value');
        return el ? el.textContent.trim() : '';
    })();

    const status = (() => {
        const el = header.querySelector('.title .nstatus');
        return el ? el.textContent.trim() : '';
    })();

    const description = (() => {
        const el = header.querySelector('.company-ai-desc .content');
        return el ? el.textContent.trim() : '';
    })();

    const industry = (() => {
        const spans = header.querySelectorAll('.main-part-item.middle .rline .f');
        for (const s of spans) {
            if (s.textContent.includes('企查查行业')) {
                return s.querySelector('.val .need-copy-field')
                    ? s.querySelector('.val .need-copy-field').textContent.trim()
                    : (s.querySelector('.val') ? s.querySelector('.val').textContent.trim() : '');
            }
        }
        return '';
    })();

    const scale = (() => {
        const spans = header.querySelectorAll('.main-part-item.middle .rline .f');
        for (const s of spans) {
            if (s.textContent.includes('企业规模')) {
                const val = s.querySelector('.val');
                return val ? val.textContent.trim() : '';
            }
        }
        return '';
    })();

    const employeeCount = (() => {
        const spans = header.querySelectorAll('.main-part-item.middle .rline .f');
        for (const s of spans) {
            if (s.textContent.includes('员工人数')) {
                const el = s.querySelector('.need-copy-field .m-r-7');
                return el ? el.textContent.trim() : '';
            }
        }
        return '';
    })();

    const financingStage = (() => {
        const items = header.querySelectorAll('.oxin-access-item');
        for (const item of items) {
            const nameEl = item.querySelector('.oxin-name');
            if (nameEl && nameEl.textContent.includes('产品信息')) {
                const stage = item.querySelector('.text-primary');
                return stage ? stage.textContent.trim() : '';
            }
        }
        return '';
    })();

    const contactInfo = header.querySelector('.contact-info');
    const leftPart = contactInfo ? contactInfo.querySelector('.main-part-item.left') : null;
    const rightPart = contactInfo ? contactInfo.querySelector('.main-part-item.right') : null;

    const legalPerson = (() => {
        if (!leftPart) return '';
        const el = leftPart.querySelector('.partner a');
        return el ? el.textContent.trim() : '';
    })();

    const registeredCapital = (() => {
        if (!leftPart) return '';
        const spans = leftPart.querySelectorAll('.rline .f');
        for (const s of spans) {
            if (s.textContent.includes('注册资本')) {
                const val = s.querySelector('.copy-value');
                return val ? val.textContent.trim() : '';
            }
        }
        return '';
    })();

    const establishedDate = (() => {
        if (!leftPart) return '';
        const spans = leftPart.querySelectorAll('.rline .f');
        for (const s of spans) {
            if (s.textContent.includes('成立日期')) {
                const val = s.querySelector('.copy-value');
                return val ? val.textContent.trim() : '';
            }
        }
        return '';
    })();

    const phone = (() => {
        if (!rightPart) return '';
        const spans = rightPart.querySelectorAll('.rline .f');
        for (const s of spans) {
            if (s.textContent.includes('电话')) {
                const val = s.querySelector('.copy-value');
                return val ? val.textContent.trim() : '';
            }
        }
        return '';
    })();

    const website = (() => {
        if (!rightPart) return '';
        const spans = rightPart.querySelectorAll('.rline .f');
        for (const s of spans) {
            if (s.textContent.includes('官网')) {
                const val = s.querySelector('.copy-value');
                return val ? val.textContent.trim() : '';
            }
        }
        return '';
    })();

    const address = (() => {
        if (!rightPart) return '';
        const spans = rightPart.querySelectorAll('.rline .f');
        for (const s of spans) {
            if (s.textContent.includes('地址')) {
                const val = s.querySelector('.copy-value');
                return val ? val.textContent.trim() : '';
            }
        }
        return '';
    })();

    // 从工商信息表格提取参保人数和经营范围
    const cominfo = document.querySelector('.cominfo');
    const insuranceCount = (() => {
        if (!cominfo) return '';
        const rows = cominfo.querySelectorAll('tr');
        for (const row of rows) {
            if (row.textContent.includes('参保人数')) {
                const cols = row.querySelectorAll('td');
                for (const col of cols) {
                    if (col.querySelector('a') && col.textContent.match(/\d+/)) {
                        const m = col.textContent.match(/(\d+)/);
                        return m ? m[1] : '';
                    }
                }
            }
        }
        return '';
    })();

    const businessScope = (() => {
        if (!cominfo) return '';
        const rows = cominfo.querySelectorAll('tr');
        for (const row of rows) {
            const tb = row.querySelector('.tb');
            if (tb && tb.textContent.includes('经营范围')) {
                const val = row.querySelector('.break-word .copy-value');
                return val ? val.textContent.trim() : '';
            }
        }
        return '';
    })();

    return JSON.stringify({
        name, status, description, industry, scale,
        employee_count: employeeCount, insurance_count: insuranceCount,
        business_scope: businessScope, established_date: establishedDate,
        registered_capital: registeredCapital, legal_person: legalPerson,
        financing_stage: financingStage, phone, website, address,
    });
})()
"#;

fn parse_detail(js_result: &serde_json::Value) -> Result<CompanyDetail, anyhow::Error> {
    let json_str = js_result
        .as_str()
        .or_else(|| js_result.get("value").and_then(|v| v.as_str()))
        .context("JS 返回值不是字符串")?;

    let raw: RawDetail = serde_json::from_str(json_str).context("公司详情 JSON 解析失败")?;

    Ok(CompanyDetail {
        name: raw.name,
        status: raw.status,
        description: raw.description,
        industry: raw.industry,
        scale: raw.scale,
        employee_count: raw.employee_count,
        insurance_count: raw.insurance_count,
        business_scope: raw.business_scope,
        established_date: raw.established_date,
        registered_capital: raw.registered_capital,
        legal_person: raw.legal_person,
        financing_stage: raw.financing_stage,
        phone: raw.phone,
        website: raw.website,
        address: raw.address,
    })
}
