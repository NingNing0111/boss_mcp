---
name: boss-mcp
description: 招聘自动化 MCP 工具集，支持 Boss直聘 与 企查查 的职位搜索、岗位分析、HR沟通、企业背调、未读消息处理等自动化招聘流程。
triggers:
  - boss
  - zhipin
  - boss直聘
  - qcc
  - company
  - recruitment
  - hiring
  - candidate
  - hr
  - interview
  - 招聘
  - 求职
  - 找工作
  - 岗位
  - JD
  - 公司查询
argument-hint: "<tool_name> [args]"
---

# Boss MCP Skill

## Purpose

该 Skill 用于自动化执行招聘相关任务，包括：

- 搜索职位
- 获取岗位 JD
- 批量筛选岗位
- 主动联系 HR
- 获取未读消息
- 跟进聊天记录
- 查询企业工商信息
- 企业背景调查

适用于：

- AI 求职 Agent
- 自动投递 Agent
- 招聘助手
- 猎头自动化
- 候选人筛选流程
- 企业背调流程

---

# Core Rules

## 登录前置规则（必须遵守）

在调用任何 Boss直聘 或 企查查工具之前：

1. 必须先检查登录状态
2. 如果未登录：
   - 调用 `boss_login`
   - 或 `qcc_login`
3. 等待用户扫码完成登录后，再继续后续流程

禁止在未登录状态下直接调用业务工具。

---

# Tool Dependency Rules

## Boss 直聘工具依赖关系

### send_message / send_resume 使用限制（重要）

`send_message`、`send_resume` 只能在`get_chat_messages`后调用。

因为只有这些工具会进入聊天页面。

如果当前不在聊天页：
- 禁止调用 `send_message`
- 禁止调用 `send_resume`
- 必须先进入聊天页

其中：
- `send_message` 用于发送文本消息，**在调用该工具前.务必调用`get_chat_messages`获取沟通的消息记录，避免发送的文本无上下文语境**
- `send_resume` 用于发送在线简历，HR主动求简历时可以用

---

## 岗位详情获取规则

`search_positions` 仅返回：

- 岗位基础信息
- 公司信息
- 薪资
- detail_url

不会返回完整 JD。

如果用户需要：
- 岗位描述
- 任职要求
- 技术栈
- 福利信息

必须继续调用：

`get_job_detail(detail_url)`

---

# Recommended Workflow

## 场景一：职位搜索与沟通（最常用）

### Step 1：检查登录

- `check_boss_login`

未登录时：
- `boss_login`

---

### Step 2：搜索岗位

使用：

- `search_positions(search_url)`

搜索 URL 应包含：

- 城市
- 职位关键词
- 薪资范围
- 经验要求
- 学历要求
- 融资阶段
- 公司规模

必要时先调用 Lookup 工具获取编码。

---

### Step 3：获取岗位详情

对目标岗位调用：

- `get_job_detail(detail_url)`

用于分析：

- 岗位要求
- 技术栈
- 工作内容
- 匹配度

---

### Step 4：主动联系 HR

直接向该岗位的hr发送首次沟通内容、问候等：

- `start_new_chat(detail_url, greeting)`


如 HR 明确索要简历，可继续：

- `send_resume()`

---

### Step 5：跟进未读消息

- `get_unread_chats`

然后：

- `get_chat_messages(idx)`

最后按需要执行：

- `send_message(message)`
- `send_resume()`

---

# QCC Workflow（企业背调）

## Step 1：检查登录

- `check_qcc_login`

未登录时：

- `qcc_login`

---

## Step 2：搜索企业

- `search_qcc_company(keyword)`

返回：

- 企业列表
- 企业名称
- detail_url

---

## Step 3：获取企业详情

- `get_qcc_company_detail(detail_url)`

用于获取：

- 工商信息
- 融资信息
- 风险信息
- 股东信息
- 企业规模
- 法律风险

---

# Lookup Tools

这些工具用于构建合法的搜索参数。

| Tool | Usage |
|------|------|
| `search_cities` | 城市名称 → 城市代码 |
| `search_industries` | 行业名称 → 行业代码 |
| `search_position_codes` | 职位名称 → 职位代码 |
| `list_filter_group` | 查询薪资/经验/学历/规模/融资筛选项 |

---

# MCP Tools

## Boss直聘

| Tool | Description |
|------|-------------|
| `check_boss_login` | 检查 Boss 登录状态 |
| `boss_login` | Boss 登录 |
| `search_positions` | 搜索职位列表 |
| `get_job_detail` | 获取岗位 JD |
| `start_new_chat` | 向岗位的HR发起首次对话 |
| `send_message` | 向 HR 发送消息 |
| `get_unread_chats` | 获取未读消息 |
| `get_chat_messages` | 获取聊天记录 |
|`send_resume`|发送简历附件,一般在`get_chat_messages`后 且 HR请求申请获取简历时可以使用|

---

## 企查查

| Tool | Description |
|------|-------------|
| `check_qcc_login` | 检查 QCC 登录状态 |
| `qcc_login` | QCC 登录 |
| `search_qcc_company` | 搜索企业 |
| `get_qcc_company_detail` | 获取企业详情 |

---

## Lookup

| Tool | Description |
|------|-------------|
| `search_cities` | 搜索城市编码 |
| `search_industries` | 搜索行业编码 |
| `search_position_codes` | 搜索职位编码 |
| `list_filter_group` | 获取筛选配置 |

---

# Agent Best Practices

## 优先使用岗位详情再做决策

不要仅根据职位列表决定是否投递。

应先：

1. 获取岗位详情
2. 分析 JD
3. 判断匹配度
4. 再联系 HR

---

## 推荐自动化行为

Agent 可以：

- 批量搜索岗位
- 自动筛选高匹配职位
- 自动生成首轮沟通话术
- 自动查询企业风险
- 自动跟进未读消息
- 自动整理候选岗位列表

---

## 不推荐行为

避免：

- 高频发送重复消息
- 未读取 JD 就直接沟通
- 未登录情况下调用业务工具
- 在非聊天页调用 send_message

---

# Runtime Notes

- Boss 与 QCC 使用独立 Tab：
  - `boss_tab`
  - `qcc_tab`

- 登录状态通过 `user_data_dir` 持久化

- 部分 QCC 企业数据需要会员权限

- 某些页面可能存在风控或验证码
