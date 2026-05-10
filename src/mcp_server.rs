use rmcp::{
    handler::server::wrapper::Parameters,
    model::{CallToolResult, Content, ServerCapabilities, ServerInfo},
    schemars,
    tool,
    tool_router,
    ErrorData,
    ServerHandler,
};
use serde::Deserialize;

use crate::{
    boss::handler::{
        get_unread_chat, get_unread_chat_message, position_detail, search_position,
        start_chat, login as boss_login, login_check as boss_login_check,
    },
    qcc::handler::{
        company_detail, login as qcc_login, login_check as qcc_login_check, search_company,
    },
    utils::{
        conditions,
        industry, position, site,
    },
    config::AppConfig,
};

#[derive(Clone)]
pub struct RecruitmentServer {
    tool_router: rmcp::handler::server::router::tool::ToolRouter<RecruitmentServer>,
}

impl RecruitmentServer {
    pub fn new() -> Self {
        Self {
            tool_router: Self::tool_router(),
        }
    }
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct LoginParams {
    pub login_type: crate::boss::model::LoginType,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct SearchJobParams {
    pub search_url: String,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct GetDetailParams {
    pub detail_url: String,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct GetChatMessageParams {
    pub idx: usize,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct FilterKeywordParams {
    pub keyword: String,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct FilterGroupParams {
    pub group_name: String,
}

#[tool_router]
impl RecruitmentServer {
    // ... existing search_positions ...
    #[tool(description = "搜索 Boss 直聘平台上的职位信息。\
        \n\n**功能说明**：\
        \n- 根据提供的搜索 URL 获取职位列表\
        \n- 返回职位基本信息，包括职位名称、公司名称、薪资范围、工作地点、职位标签等\
        \n\n**使用场景**：\
        \n- 初始化职位搜索\
        \n- 获取特定筛选条件下的职位列表\
        \n\n**返回数据**：职位列表的 JSON 数组，每个职位包含详细信息")]
    async fn search_positions(&self, Parameters(params): Parameters<SearchJobParams>) -> Result<CallToolResult, ErrorData> {
        let positions = search_position(&params.search_url)
            .map_err(|e| ErrorData::internal_error(e.to_string(), None))?;
        Ok(CallToolResult::success(vec![Content::text(serde_json::to_string(&positions).unwrap())]))
    }

    #[tool(description = "发起 Boss 直聘登录流程，支持扫码登录和验证码登录两种方式。\
        \n\n**功能说明**：\
        \n- login_type 参数指定登录方式：QR_CODE（扫码登录）或 VERIFICATION_CODE（验证码登录）\
        \n- 扫码登录：生成二维码图片，使用 Boss 直聘 APP 扫描完成认证\
        \n- 验证码登录：使用手机号码接收短信验证码完成认证\
        \n\n**使用场景**：\
        \n- 首次使用系统时需要进行身份认证\
        \n- 登录状态过期需要重新认证\
        \n\n**返回值**：返回登录二维码图片的保存路径，用户可通过该路径查看二维码")]
    async fn boss_login(&self, Parameters(params): Parameters<LoginParams>) -> Result<CallToolResult, ErrorData> {
        let config = AppConfig::default();
        let path = boss_login(params.login_type, &config)
            .map_err(|e| ErrorData::internal_error(e.to_string(), None))?;
        Ok(CallToolResult::success(vec![Content::text(format!("Login QR code saved to: {:?}", path))]))
    }

    #[tool(description = "检查当前 Boss 直聘的登录状态。\
        \n\n**功能说明**：\
        \n- 验证当前浏览器/会话是否已成功登录 Boss 直聘\
        \n- 返回布尔值表示当前认证状态\
        \n\n**使用场景**：\
        \n- 执行需要登录权限的操作前检查认证状态\
        \n- 批量操作前确认会话有效性\
        \n- 自动判断是否需要重新登录\
        \n\n**返回值**：true 表示已登录，false 表示未登录")]
    async fn check_boss_login(&self) -> Result<CallToolResult, ErrorData> {
        let is_logged = boss_login_check()
            .map_err(|e| ErrorData::internal_error(e.to_string(), None))?;
        Ok(CallToolResult::success(vec![Content::text(format!("Logged in: {}", is_logged))]))
    }

    #[tool(description = "发起企查查平台扫码登录流程。\
        \n\n**功能说明**：\
        \n- 生成二维码图片，使用企查查 APP 或微信扫码完成认证\
        \n- 企查查是国内领先的企业信息查询平台\
        \n\n**使用场景**：\
        \n- 首次使用时进行身份认证\
        \n- 查询企业信息前确保登录状态有效\
        \n- 企查查会话过期需重新登录\
        \n\n**返回值**：返回登录二维码图片的保存路径")]
    async fn qcc_login(&self) -> Result<CallToolResult, ErrorData> {
        let config = AppConfig::default();
        let path = qcc_login(&config)
            .map_err(|e| ErrorData::internal_error(e.to_string(), None))?;
        Ok(CallToolResult::success(vec![Content::text(format!("Login QR code saved to: {:?}", path))]))
    }

    #[tool(description = "检查当前企查查平台的登录状态。\
        \n\n**功能说明**：\
        \n- 验证当前会话是否已成功登录企查查\
        \n- 返回布尔值表示当前认证状态\
        \n\n**使用场景**：\
        \n- 查询企业信息前检查认证状态\
        \n- 批量查询前确认会话有效性\
        \n- 自动判断是否需要重新扫码登录\
        \n\n**返回值**：true 表示已登录，false 表示未登录")]
    async fn check_qcc_login(&self) -> Result<CallToolResult, ErrorData> {
        let is_logged = qcc_login_check()
            .map_err(|e| ErrorData::internal_error(e.to_string(), None))?;
        Ok(CallToolResult::success(vec![Content::text(format!("Logged in: {}", is_logged))]))
    }

    #[tool(description = "通过关键词搜索企查查平台上的企业信息。\
        \n\n**功能说明**：\
        \n- 根据 keyword 参数搜索匹配的企业\
        \n- 支持搜索：公司名称、品牌名称、法人代表、统一社会信用代码等\
        \n- 返回企业列表包含：\
        \n  - 企业名称、统一社会信用代码\
        \n  - 法定代表人、注册资本\
        \n  - 成立日期、企业状态（存续、吊销等）\
        \n  - 注册地址、经营范围\
        \n\n**使用场景**：\
        \n- 背调候选人所在公司信息\
        \n- 核实企业真实性\
        \n- 查询目标公司工商信息\
        \n\n**注意**：部分企业详细信息可能需要付费会员权限")]
    async fn search_qcc_company(&self, Parameters(params): Parameters<FilterKeywordParams>) -> Result<CallToolResult, ErrorData> {
        let res = search_company(&params.keyword)
            .map_err(|e| ErrorData::internal_error(e.to_string(), None))?;
        Ok(CallToolResult::success(vec![Content::text(serde_json::to_string(&res).unwrap())]))
    }

    #[tool(description = "获取企查查平台公司的详细信息。\
        \n\n**功能说明**：\
        \n- 根据公司详情 URL 获取完整的企业信息\
        \n- 需要先通过 search_qcc_company 获取公司列表，再使用对应公司的详情链接\
        \n- 返回数据包括：\
        \n  - 企业基本信息：名称、统一社会信用代码、法定代表人、注册资本\
        \n  - 工商信息：成立日期、经营状态、企业类型、核准日期\
        \n  - 地址信息：注册地址、实际经营地址\
        \n  - 经营信息：经营范围、营业期限、参保人数\
        \n  - 股东信息、主要人员、对外投资\
        \n  - 变更记录、分支机构\
        \n\n**使用场景**：\
        \n- 查看企业完整工商档案\
        \n- 背调核实公司背景\
        \n- 分析目标公司的股权结构\
        \n- 了解企业经营状况\
        \n\n**注意**：部分敏感信息（如联系方式、详细股东信息）需要付费会员权限")]
    async fn get_qcc_company_detail(&self, Parameters(params): Parameters<GetDetailParams>) -> Result<CallToolResult, ErrorData> {
        let detail = company_detail(&params.detail_url)
            .map_err(|e| ErrorData::internal_error(e.to_string(), None))?;
        Ok(CallToolResult::success(vec![Content::text(serde_json::to_string(&detail).unwrap())]))
    }

    #[tool(description = "获取 Boss 直聘职位的详细信息。\
        \n\n**功能说明**：\
        \n- 根据职位详情页 URL 获取完整的职位信息\
        \n- 返回数据包括：\
        \n  - 职位名称、职位描述、任职要求\
        \n  - 薪资范围（工资区间）\
        \n  - 工作地点（城市、区县、具体地址）\
        \n  - 经验要求、学历要求、招聘人数\
        \n  - 职位标签、技能要求\
        \n  - 公司基本信息（规模、行业、融资阶段）\
        \n  - 招聘者信息（HR/ recruiter 姓名、职位）\
        \n\n**使用场景**：\
        \n- 用户点击职位后查看完整信息\
        \n- 筛选候选人前确认职位详情\
        \n- 生成职位分析报告")]
    async fn get_job_detail(&self, Parameters(params): Parameters<GetDetailParams>) -> Result<CallToolResult, ErrorData> {
        let detail = position_detail(&params.detail_url)
            .map_err(|e| ErrorData::internal_error(e.to_string(), None))?;
        Ok(CallToolResult::success(vec![Content::text(serde_json::to_string(&detail).unwrap())]))
    }

    #[tool(description = "获取 Boss 直聘平台中的未读聊天消息。\
        \n\n**功能说明**：\
        \n- 获取当前账号所有未读消息的会话列表\
        \n- 每个会话包含：\
        \n  - 对方名称（HR/recruiter 或求职者）\
        \n  - 最近一条消息内容\
        \n  - 未读消息数量\
        \n  - 会话时间\
        \n\n**使用场景**：\
        \n- 查看有哪些 HR 联系过你\
        \n- 了解未回复的沟通请求\
        \n- 快速查看所有待处理沟通\
        \n\n**注意**：此操作需要有效的登录状态才能执行")]
    async fn get_unread_chats(&self) -> Result<CallToolResult, ErrorData> {
        let chats = get_unread_chat()
            .map_err(|e| ErrorData::internal_error(e.to_string(), None))?;
        Ok(CallToolResult::success(vec![Content::text(serde_json::to_string(&chats).unwrap())]))
    }

    #[tool(description = "获取指定未读会话的完整聊天记录。\
        \n\n**功能说明**：\
        \n- idx 参数指定会话索引（从 0 开始），对应 get_unread_chats 返回的会话列表\
        \n- 返回该会话的所有历史消息\
        \n- 每条消息包含：发送者、消息内容、发送时间、消息类型（文本/图片/语音等）\
        \n\n**使用场景**：\
        \n- 点击某个未读会话查看完整对话\
        \n- 了解与 HR 的完整沟通历史\
        \n- 分析候选人与招聘方的沟通内容\
        \n\n**注意**：需要先调用 get_unread_chats 获取会话索引")]
    async fn get_chat_messages(&self, Parameters(params): Parameters<GetChatMessageParams>) -> Result<CallToolResult, ErrorData> {
        let msgs = get_unread_chat_message(params.idx)
            .map_err(|e| ErrorData::internal_error(e.to_string(), None))?;
        Ok(CallToolResult::success(vec![Content::text(serde_json::to_string(&msgs).unwrap())]))
    }

    #[tool(description = "向招聘方发起新聊天对话。\
        \n\n**功能说明**：\
        \n- 根据职位详情 URL 直接发起与 HR/recruiter 的沟通\
        \n- 可用于主动投递简历后的跟进沟通\
        \n- 支持查看职位详情页后直接发起聊天\
        \n\n**使用场景**：\
        \n- 主动联系感兴趣的职位 HR\
        \n- 对有意向的职位发起初步沟通\
        \n- 跟进已投递的职位申请\
        \n\n**注意**：需要有效的登录状态，部分热门职位可能有发送频率限制")]
    async fn start_new_chat(&self, Parameters(params): Parameters<GetDetailParams>) -> Result<CallToolResult, ErrorData> {
        start_chat(&params.detail_url)
            .map_err(|e| ErrorData::internal_error(e.to_string(), None))?;
        Ok(CallToolResult::success(vec![Content::text("Started chat")]))
    }

    // New Filter Tools
    #[tool(description = "搜索 Boss 直聘支持的城市列表。\
        \n\n**功能说明**：\
        \n- 根据关键词搜索匹配的城市\
        \n- 支持中文城市名称模糊匹配\
        \n- 返回城市代码和城市名称映射\
        \n\n**使用场景**：\
        \n- 构建职位搜索筛选条件\
        \n- 获取城市代码用于 URL 构建\
        \n- 用户选择目标工作城市")]
    async fn search_cities(&self, Parameters(params): Parameters<FilterKeywordParams>) -> Result<CallToolResult, ErrorData> {
        let res = site::search_cities(&params.keyword);
        Ok(CallToolResult::success(vec![Content::text(serde_json::to_string(&res).unwrap())]))
    }

    #[tool(description = "搜索 Boss 直聘支持的行业分类。\
        \n\n**功能说明**：\
        \n- 根据关键词搜索匹配的行业分类\
        \n- 支持行业名称模糊搜索\
        \n- 返回行业代码和行业名称映射\
        \n\n**使用场景**：\
        \n- 设置职位搜索的行业筛选条件\
        \n- 获取行业代码用于精准搜索\
        \n- 按行业筛选候选人\
        \n\n**常见行业**：互联网、金融、房地产、教育、医疗、制造等")]
    async fn search_industries(&self, Parameters(params): Parameters<FilterKeywordParams>) -> Result<CallToolResult, ErrorData> {
        let res = industry::search_industries(&params.keyword);
        Ok(CallToolResult::success(vec![Content::text(serde_json::to_string(&res).unwrap())]))
    }

    #[tool(description = "搜索 Boss 直聘支持的职位名称/职能分类。\
        \n\n**功能说明**：\
        \n- 根据关键词搜索匹配的职位名称\
        \n- 支持职位名称模糊搜索\
        \n- 返回职位代码和职位名称映射\
        \n\n**使用场景**：\
        \n- 设置职位搜索的职能筛选条件\
        \n- 获取职位代码用于构建精确搜索 URL\
        \n- 按职位类型筛选候选人\
        \n\n**常见职位**：Java 开发、Python 开发、前端开发、产品经理、设计师等")]
    async fn search_position_codes(&self, Parameters(params): Parameters<FilterKeywordParams>) -> Result<CallToolResult, ErrorData> {
        let res = position::search_positions(&params.keyword);
        Ok(CallToolResult::success(vec![Content::text(serde_json::to_string(&res).unwrap())]))
    }

    #[tool(description = "获取指定筛选维度的所有可选值。\
        \n\n**功能说明**：\
        \n- 根据 group_name 获取特定筛选维度的完整选项列表\
        \n- 支持的筛选维度包括：\
        \n  - salary: 薪资范围选项（如 5k以下、5k-10k、10k-20k 等）\
        \n  - experience: 经验要求选项（如 应届生、1-3年、3-5年、5-10年等）\
        \n  - education: 学历要求选项（如 大专、本科、硕士、博士）\
        \n  - company_size: 公司规模选项（如 0-20人、20-99人、100-499人等）\
        \n  - financing: 融资阶段选项（如 未融资、天使轮、A轮、B轮、上市公司等）\
        \n\n**使用场景**：\
        \n- 构建高级筛选条件的下拉选项\
        \n- 获取所有可选值用于多选筛选\
        \n- 生成筛选 UI 的选项列表")]
    async fn list_filter_group(&self, Parameters(params): Parameters<FilterGroupParams>) -> Result<CallToolResult, ErrorData> {
        let res = conditions::list_filter_group(&params.group_name);
        Ok(CallToolResult::success(vec![Content::text(serde_json::to_string(&res).unwrap())]))
    }
}

#[rmcp::tool_handler(router = self.tool_router)]
impl ServerHandler for RecruitmentServer {
    fn get_info(&self) -> ServerInfo {
        ServerInfo::new(
            ServerCapabilities::builder()
                .enable_tools()
                .build(),
        )
    }
}
