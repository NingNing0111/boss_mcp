use serde::{Deserialize, Serialize};
use serde_json::Value;

// Boss 登录类型
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LoginType {
    // 手机号登录
    Phone,
    // 二维码登录
    QRCode,
}

// Boss 账户信息
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BossAccountInfo {
    // 展示的姓名
    show_name: String,
    // 性别
    gender: i64,
    // 昵称
    name: String,
    // 头像
    avatar: String,
    // HR职位
    title: String,
    // comId
    com_id: i64,
    // 加密的comId
    encrypt_com_id: String,
    is_648_vip: bool,
}

impl BossAccountInfo {
    pub fn from_base_info(base_info: &Value, is_648_vip: bool) -> Result<Self, anyhow::Error> {
        fn required_str(v: &Value, key: &str) -> Result<String, anyhow::Error> {
            v.get(key)
                .and_then(|x| x.as_str())
                .map(|s| s.to_string())
                .ok_or_else(|| anyhow::anyhow!("missing baseInfo.{key}"))
        }

        fn required_i64(v: &Value, key: &str) -> Result<i64, anyhow::Error> {
            v.get(key)
                .and_then(|x| x.as_i64())
                .ok_or_else(|| anyhow::anyhow!("missing baseInfo.{key}"))
        }

        Ok(BossAccountInfo {
            show_name: required_str(base_info, "showName")?,
            gender: required_i64(base_info, "gender")?,
            name: required_str(base_info, "name")?,
            avatar: required_str(base_info, "avatar")?,
            title: required_str(base_info, "title")?,
            com_id: required_i64(base_info, "comId")?,
            encrypt_com_id: required_str(base_info, "encryptComId")?,
            is_648_vip,
        })
    }

    pub fn show_name(&self) -> &str {
        &self.show_name
    }

    pub fn gender(&self) -> i64 {
        self.gender
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn avatar(&self) -> &str {
        &self.avatar
    }

    pub fn title(&self) -> &str {
        &self.title
    }

    pub fn com_id(&self) -> i64 {
        self.com_id
    }

    pub fn encrypt_com_id(&self) -> &str {
        &self.encrypt_com_id
    }

    pub fn is_648_vip(&self) -> bool {
        self.is_648_vip
    }
}
