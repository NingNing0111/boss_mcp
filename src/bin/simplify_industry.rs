use std::{fs, path::Path};

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

const INPUT_PATH: &str = "src/resource/industry.json";
const OUTPUT_PATH: &str = "src/resource/industry.simple.json";

#[derive(Debug, Deserialize)]
struct IndustryNode {
    code: u64,
    name: String,
    #[serde(rename = "subLevelModelList")]
    sub_level_model_list: Option<Vec<IndustryNode>>,
}

#[derive(Debug, Serialize)]
struct SimpleIndustryNode {
    code: u64,
    name: String,
    #[serde(rename = "subLevelModelList", skip_serializing_if = "Option::is_none")]
    sub_level_model_list: Option<Vec<SimpleIndustryNode>>,
}

fn main() -> Result<()> {
    let source = fs::read_to_string(INPUT_PATH)
        .with_context(|| format!("读取行业文件失败: {INPUT_PATH}"))?;
    let nodes: Vec<IndustryNode> =
        serde_json::from_str(&source).with_context(|| format!("解析行业文件失败: {INPUT_PATH}"))?;

    let simplified: Vec<SimpleIndustryNode> =
        nodes.into_iter().map(SimpleIndustryNode::from).collect();
    let output = serde_json::to_string_pretty(&simplified).context("序列化简化行业数据失败")?;

    if let Some(parent) = Path::new(OUTPUT_PATH).parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("创建输出目录失败: {}", parent.display()))?;
    }

    fs::write(OUTPUT_PATH, output).with_context(|| format!("写入简化文件失败: {OUTPUT_PATH}"))?;
    println!("已生成简化行业文件: {OUTPUT_PATH}");

    Ok(())
}

impl From<IndustryNode> for SimpleIndustryNode {
    fn from(node: IndustryNode) -> Self {
        Self {
            code: node.code,
            name: node.name,
            sub_level_model_list: node
                .sub_level_model_list
                .map(|children| children.into_iter().map(SimpleIndustryNode::from).collect()),
        }
    }
}
