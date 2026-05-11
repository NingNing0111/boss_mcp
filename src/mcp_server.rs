use std::path::Path;

use rmcp::{
    ErrorData, ServerHandler,
    handler::server::wrapper::Parameters,
    model::{CallToolResult, Content, ServerCapabilities, ServerInfo},
    schemars, tool, tool_router,
};
use serde::{Deserialize, Serialize};

use crate::{
    boss::handler::{
        get_unread_chat, get_unread_chat_message, login as boss_login,
        login_check as boss_login_check, position_detail, search_position, send_greeting_message,
        send_resume, start_chat,
    },
    browser,
    config::load_or_create,
    qcc::handler::{
        company_detail, login as qcc_login, login_check as qcc_login_check, search_company,
    },
    utils::{conditions, industry, position, site},
};

const BOSS_TAB_ID: &str = "boss_tab";
const QCC_TAB_ID: &str = "qcc_tab";

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
    pub greeting: String,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct GetChatMessageParams {
    pub idx: usize,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct SendMessageParams {
    pub message: String,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct FilterKeywordParams {
    pub keyword: String,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct FilterGroupParams {
    pub group_name: String,
}

#[derive(Debug, Serialize)]
struct ChatPageReadyResponse<'a> {
    status: &'a str,
    tab: &'a str,
    detail_url: &'a str,
}

#[derive(Debug, Serialize)]
struct ChatMessagesResponse<T> {
    tab: &'static str,
    messages: T,
}

#[derive(Debug, Serialize)]
struct SendMessageResponse<'a> {
    success: bool,
    tab: &'a str,
    message: &'a str,
}

fn json_text<T: Serialize>(value: &T) -> Result<CallToolResult, ErrorData> {
    let text =
        serde_json::to_string(value).map_err(|e| ErrorData::internal_error(e.to_string(), None))?;
    Ok(CallToolResult::success(vec![Content::text(text)]))
}

fn validate_keyword(keyword: &str) -> Result<&str, ErrorData> {
    let keyword = keyword.trim();
    if keyword.is_empty() {
        return Err(ErrorData::internal_error(
            "keyword 不能为空，请提供更具体的搜索词".to_string(),
            None,
        ));
    }
    Ok(keyword)
}

#[tool_router]
impl RecruitmentServer {
    #[tool(description = "检查当前 Boss 直聘登录状态。\
        \n\n调用规则：任何 Boss 业务工具前都应先调用本工具；未登录时先执行 boss_login，等用户扫码或验证码完成后再继续。\
        \n\n返回：JSON 字符串，包含 logged_in 与 tab=boss_tab。")]
    async fn check_boss_login(&self) -> Result<CallToolResult, ErrorData> {
        let is_logged =
            boss_login_check().map_err(|e| ErrorData::internal_error(e.to_string(), None))?;
        json_text(&serde_json::json!({
            "logged_in": is_logged,
            "tab": BOSS_TAB_ID,
        }))
    }

    #[tool(description = "发起 Boss 直聘登录。\
        \n\n参数：login_type，可选 Phone 或 QRCode。\
        \n\n调用时机：仅在 check_boss_login 返回未登录时调用。\
        \n\n返回：JSON 字符串，包含二维码或登录流程输出路径，以及 tab=boss_tab。")]
    async fn boss_login(
        &self,
        Parameters(params): Parameters<LoginParams>,
    ) -> Result<CallToolResult, ErrorData> {
        let config = load_or_create(Path::new("config.yaml"))
            .map_err(|e| ErrorData::internal_error(e.to_string(), None))?;
        let path = boss_login(params.login_type, &config)
            .map_err(|e| ErrorData::internal_error(e.to_string(), None))?;
        json_text(&serde_json::json!({
            "tab": BOSS_TAB_ID,
            "login_output_path": path,
        }))
    }

    #[tool(description = "搜索 Boss 直聘职位列表。\
        \n\n前置条件：必须先确认已登录。\
        \n\n参数：search_url，必须是完整的 Boss 搜索页 URL；城市、行业、职位、薪资、经验、学历、公司规模、融资阶段等编码可先通过 lookup 工具获取后自行拼接。\
        \n\n返回：职位列表 JSON。仅包含基础信息与 detail_url，不包含完整 JD；如果要看职位描述、任职要求、技术栈、招聘者信息，必须继续调用 get_job_detail(detail_url)。")]
    async fn search_positions(
        &self,
        Parameters(params): Parameters<SearchJobParams>,
    ) -> Result<CallToolResult, ErrorData> {
        let positions = search_position(&params.search_url)
            .map_err(|e| ErrorData::internal_error(e.to_string(), None))?;
        json_text(&positions)
    }

    #[tool(description = "获取 Boss 直聘职位详情。\
        \n\n前置条件：必须先确认已登录；detail_url 应来自 search_positions 返回结果。\
        \n\n用途：用于补全完整 JD，包括职位描述、关键词、招聘者姓名、招聘者职位、活跃时间、公司信息。\
        \n\n建议：不要只根据职位列表决定是否沟通，先读取职位详情再决定是否联系 HR。")]
    async fn get_job_detail(
        &self,
        Parameters(params): Parameters<GetDetailParams>,
    ) -> Result<CallToolResult, ErrorData> {
        let detail = position_detail(&params.detail_url)
            .map_err(|e| ErrorData::internal_error(e.to_string(), None))?;
        json_text(&detail)
    }

    #[tool(description = "发起岗位沟通 greeting 是首次沟通发送文本 \
        \n\n前置条件：必须先确认已登录；detail_url 应来自 search_positions。\
        \n\n返回：JSON 字符串，包含 status=ready_to_send_message、detail_url、tab=boss_tab。")]
    async fn start_new_chat(
        &self,
        Parameters(params): Parameters<GetDetailParams>,
    ) -> Result<CallToolResult, ErrorData> {
        match start_chat(&params.detail_url, &params.greeting)
            .map_err(|e| ErrorData::internal_error(e.to_string(), None))
        {
            Ok(_) => {
                return json_text(&ChatPageReadyResponse {
                    status: "ready_to_send_message",
                    tab: BOSS_TAB_ID,
                    detail_url: &params.detail_url,
                });
            }

            Err(e) => {
                return json_text(&ChatPageReadyResponse {
                    status: &format!("Error:{}", e.to_string()),
                    tab: BOSS_TAB_ID,
                    detail_url: &params.detail_url,
                });
            }
        }
    }

    #[tool(description = "向当前 Boss 聊天页发送文本消息。\
        \n\n严格前置条件：只能在 start_new_chat 或 get_chat_messages 成功之后调用；如果当前不在聊天页，禁止调用。\
        \n\n参数：message，为要发送的文本内容。\
        \n\n返回：JSON 字符串，包含 success、message、tab=boss_tab。")]
    async fn send_message(
        &self,
        Parameters(params): Parameters<SendMessageParams>,
    ) -> Result<CallToolResult, ErrorData> {
        let success = browser::with_boss_tab(|page| send_greeting_message(page, &params.message))
            .map_err(|e| ErrorData::internal_error(e.to_string(), None))?;
        json_text(&SendMessageResponse {
            success,
            tab: BOSS_TAB_ID,
            message: &params.message,
        })
    }

    #[tool(description = "发送在线简历给当前 Boss 聊天对象。\
        \n\n严格前置条件：只能在 start_new_chat 或 get_chat_messages 成功之后调用，如果当前不在聊天页，禁止调用，且只有双方都回复了才能调用\
        \n\n推荐时机：通常在 HR 明确索要简历、申请附件或希望进一步了解候选人时使用。\
        \n\n返回：JSON 字符串，包含 success 与 tab=boss_tab。")]
    async fn send_resume(&self) -> Result<CallToolResult, ErrorData> {
        let success = browser::with_boss_tab(send_resume)
            .map_err(|e| ErrorData::internal_error(e.to_string(), None))?;
        json_text(&serde_json::json!({
            "success": success,
            "tab": BOSS_TAB_ID,
        }))
    }

    #[tool(description = "获取 Boss 未读会话列表。\
        \n\n前置条件：必须先确认已登录。\
        \n\n返回：未读聊天会话数组；每项包含 idx、对方姓名、公司、头衔、未读数、最近消息、时间等字段。后续应使用 get_chat_messages(idx) 进入具体会话并读取完整消息。")]
    async fn get_unread_chats(&self) -> Result<CallToolResult, ErrorData> {
        let chats =
            get_unread_chat().map_err(|e| ErrorData::internal_error(e.to_string(), None))?;
        json_text(&serde_json::json!({
            "tab": BOSS_TAB_ID,
            "chats": chats,
        }))
    }

    #[tool(description = "进入指定未读会话并获取聊天记录。\
        \n\n前置条件：必须先调用 get_unread_chats 获取 idx。\
        \n\n重要限制：本工具会把 boss_tab 切到对应聊天页，因此执行成功后可以继续调用 send_message。\
        \n\n返回：JSON 字符串，包含 tab=boss_tab 与 messages。")]
    async fn get_chat_messages(
        &self,
        Parameters(params): Parameters<GetChatMessageParams>,
    ) -> Result<CallToolResult, ErrorData> {
        let msgs = get_unread_chat_message(params.idx)
            .map_err(|e| ErrorData::internal_error(e.to_string(), None))?;
        json_text(&ChatMessagesResponse {
            tab: BOSS_TAB_ID,
            messages: msgs,
        })
    }

    #[tool(description = "检查当前企查查登录状态。\
        \n\n调用规则：任何企查查业务工具前都应先调用本工具；未登录时先执行 qcc_login，等用户扫码完成后再继续。\
        \n\n返回：JSON 字符串，包含 logged_in 与 tab=qcc_tab。")]
    async fn check_qcc_login(&self) -> Result<CallToolResult, ErrorData> {
        let is_logged =
            qcc_login_check().map_err(|e| ErrorData::internal_error(e.to_string(), None))?;
        json_text(&serde_json::json!({
            "logged_in": is_logged,
            "tab": QCC_TAB_ID,
        }))
    }

    #[tool(description = "发起企查查扫码登录。\
        \n\n调用时机：仅在 check_qcc_login 返回未登录时调用。\
        \n\n返回：JSON 字符串，包含登录输出路径，以及 tab=qcc_tab。")]
    async fn qcc_login(&self) -> Result<CallToolResult, ErrorData> {
        let config = load_or_create(Path::new("config.yaml"))
            .map_err(|e| ErrorData::internal_error(e.to_string(), None))?;
        let path =
            qcc_login(&config).map_err(|e| ErrorData::internal_error(e.to_string(), None))?;
        json_text(&serde_json::json!({
            "tab": QCC_TAB_ID,
            "login_output_path": path,
        }))
    }

    #[tool(description = "按关键词搜索企查查企业。\
        \n\n前置条件：必须先确认已登录。\
        \n\n返回：企业列表 JSON，通常包含企业名称与 detail_url；如果需要工商、融资、风险、股东等完整信息，继续调用 get_qcc_company_detail(detail_url)。")]
    async fn search_qcc_company(
        &self,
        Parameters(params): Parameters<FilterKeywordParams>,
    ) -> Result<CallToolResult, ErrorData> {
        let res = search_company(&params.keyword)
            .map_err(|e| ErrorData::internal_error(e.to_string(), None))?;
        json_text(&res)
    }

    #[tool(description = "获取企查查企业详情。\
        \n\n前置条件：必须先确认已登录；detail_url 应来自 search_qcc_company。\
        \n\n用途：读取工商信息、融资信息、风险信息、股东信息、企业规模、法律风险等；部分字段可能受会员权限限制。")]
    async fn get_qcc_company_detail(
        &self,
        Parameters(params): Parameters<GetDetailParams>,
    ) -> Result<CallToolResult, ErrorData> {
        let detail = company_detail(&params.detail_url)
            .map_err(|e| ErrorData::internal_error(e.to_string(), None))?;
        json_text(&detail)
    }

    #[tool(description = "搜索 Boss 支持的城市编码。\
        \n\n用途：把中文城市名转换为 search_url 可用的城市 code。")]
    async fn search_cities(
        &self,
        Parameters(params): Parameters<FilterKeywordParams>,
    ) -> Result<CallToolResult, ErrorData> {
        let keyword = validate_keyword(&params.keyword)?;
        let res = site::search_cities(keyword);
        json_text(&res)
    }

    #[tool(description = "搜索 Boss 支持的行业编码。\
        \n\n用途：把行业名称转换为 search_url 可用的行业 code。")]
    async fn search_industries(
        &self,
        Parameters(params): Parameters<FilterKeywordParams>,
    ) -> Result<CallToolResult, ErrorData> {
        let keyword = validate_keyword(&params.keyword)?;
        let res = industry::search_industries(keyword);
        json_text(&res)
    }

    #[tool(description = "搜索 Boss 支持的职位编码。\
        \n\n用途：把职位名称转换为 search_url 可用的 position code。")]
    async fn search_position_codes(
        &self,
        Parameters(params): Parameters<FilterKeywordParams>,
    ) -> Result<CallToolResult, ErrorData> {
        let keyword = validate_keyword(&params.keyword)?;
        let res = position::search_positions(keyword);
        json_text(&res)
    }

    #[tool(description = "获取某个筛选维度的全部可选项。\
        \n\n支持：salary、experience、education、company_size、financing。\
        \n\n用途：辅助构建 Boss 搜索 URL。")]
    async fn list_filter_group(
        &self,
        Parameters(params): Parameters<FilterGroupParams>,
    ) -> Result<CallToolResult, ErrorData> {
        let res = conditions::list_filter_group(&params.group_name);
        json_text(&res)
    }
}

#[rmcp::tool_handler(router = self.tool_router)]
impl ServerHandler for RecruitmentServer {
    fn get_info(&self) -> ServerInfo {
        ServerInfo::new(ServerCapabilities::builder().enable_tools().build())
    }
}
