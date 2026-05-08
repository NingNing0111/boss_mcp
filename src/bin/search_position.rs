use std::path::Path;

use anyhow::anyhow;
use boss_mcp::{
    boss::handler::search_position,
    browser,
    config::load_or_create,
    utils::{
        job_query::{JobSearchParams, build_job_search_url},
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

    let result = search_position(&search_url)?;

    println!("{}", serde_json::to_string_pretty(&result)?);

    Ok(())
}
