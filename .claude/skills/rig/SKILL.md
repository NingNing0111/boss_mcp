---
name: rig
description: 在 Rust 项目中使用 Rig 库构建 LLM 应用、Agent、RAG、工具调用与向量检索时遵循本 Skill。适用于 rig / rig-core、多模型 Provider、配套向量库与可选特性（derive、rmcp、wasm 等）。
---

# Rig 库 — Code Agent 使用指南

[Rig](https://github.com/0xPlaygrounds/rig) 是用 Rust 编写的 LLM 应用库：通过**统一的 trait** 抽象多家模型厂商与多种向量存储，并提供 **Agent 构建器**、流式输出、结构化输出、工具调用、对话记忆与 OpenTelemetry GenAI 语义约定等能力。

官方文档：<https://docs.rig.rs> · API：<https://docs.rs/rig/latest/rig/>

## 何时阅读本 Skill

- 在 Rust 中接入 OpenAI、Anthropic、Ollama、Gemini 等任一 Rig 内置 Provider，或扩展自定义 Provider。
- 实现 **Agent**（系统提示、温度、工具、动态 RAG 上下文、多轮、结构化 JSON 输出）。
- 实现 **嵌入 + 向量库**（内存索引或 MongoDB / Qdrant / LanceDB 等配套 crate）。
- 定义 **Tool**（手写 `Tool` trait 或 `#[rig_tool]` 过程宏）。
- 处理 **WASM** 目标、`Send`/`Sync` 与错误类型差异。
- 选择 `rig` 门面 crate 与 `rig-core`、以及 `Cargo.toml` 中的 **feature**。

## 仓库内必读工程约束

在 Rig **源码仓库内**改代码时，还须遵守协作约定摘要：**[`reference/engineering-constraints.md`](reference/engineering-constraints.md)**（trait 复用、错误与 WASM、Provider/向量库、验证命令等）。

## 快速心智模型

1. **Provider `Client`**：实现 `ProviderClient`（如 `from_env()`），并按能力实现 `CompletionClient`、`EmbeddingsClient` 等；用 `client.completion_model(...)` / `client.embedding_model(...)` 或快捷方式 `client.agent(model_id)`。
2. **Agent**：`AgentBuilder` 或 `client.agent(...).preamble(...).tool(...).dynamic_context(n, index).build()`；高层交互用 `completion::Prompt`、`Chat`、`TypedPrompt` 等 trait（由 `Agent` 实现）。
3. **RAG**：文档类型 `#[derive(Embed)]` + `#[embed]` 标记嵌入字段 → `EmbeddingsBuilder` 批量嵌入 → `VectorStore` / `index` → `dynamic_context` 挂到 Agent。
4. **向量库**：核心 trait 在 `rig_core::vector_store`；具体后端在 **`rig-*` 配套 crate**，通过根 crate `rig` 的 **同名 feature** 暴露为 `rig::qdrant` 等模块。

## 最小代码骨架（完整片段见手册）

一次补全 + Agent（需设置对应 Provider 环境变量，如 `OPENAI_API_KEY`）：

```rust
use anyhow::Result;
use rig::client::{CompletionClient, ProviderClient};
use rig::completion::Prompt;
use rig::providers::openai;

#[tokio::main]
async fn main() -> Result<()> {
    let client = openai::Client::from_env()?;
    let agent = client
        .agent(openai::GPT_4O)
        .preamble("You are a concise assistant.")
        .build();
    println!("{}", agent.prompt("What is Rig?").await?);
    Ok(())
}
```

**可复制长示例**（Ollama、RAG、`Chat`、`prompt_typed`、流式、Memory、Hook、`#[tool_macro]`、手写 `Tool`、`top_n`、feature 片段等）集中在 **[`reference/code-examples.md`](reference/code-examples.md)**，编写应用时优先打开该文件拷贝粘贴再改。

## Cargo 依赖选择

| 场景 | 依赖 | 说明 |
|------|------|------|
| 全功能、可选向量库与 Bedrock 等 | `rig` + 所需 `features` | 根门面，`pub use rig_core::*`，并按 feature 再导出配套模块 |
| 仅核心抽象与内置 Provider | `rig-core` | 不拉配套向量/Bedrock 等 |
| 过程宏 `Embed` / `rig_tool` | `rig` 或 `rig-core` 启用 **`derive`** | `rig_core` 将 `rig_derive::rig_tool` 重导出为 `tool_macro` |

根 `rig` 的 feature 与版本以你项目依赖解析结果为准；**[`reference/dependencies-and-features.md`](reference/dependencies-and-features.md)** 中有常用 feature 表与 `Cargo.toml` 片段（文档中的版本号示例可能更新，以 crates.io 或本地 path 依赖为准）。**不要**凭记忆编造 feature 名或模块路径。

## 模块与类型导航（编程时优先查的路径）

- 客户端与能力：`rig::client`（`ProviderClient`、`CompletionClient`、`EmbeddingsClient` 等）
- 补全与消息：`rig::completion`（`Prompt`、`Chat`、`TypedPrompt`、`Completion`、`CompletionModel`）
- 嵌入：`rig::embeddings`（`EmbeddingModel`、`EmbeddingsBuilder`）
- Agent：`rig::agent`（`Agent`、`AgentBuilder`）
- 工具：`rig::tool`、`rig::tools`
- 向量：`rig::vector_store`（`VectorStoreIndex`、`InsertDocuments`、`in_memory_store` 等）
- 流式：`rig::streaming`
- 厂商：`rig::providers::*`（`openai`、`anthropic`、`ollama` 等）
- 遥测：`rig::telemetry`

## 详细参考（按需展开）

以下文件按主题拆分，**实现代码前应查阅与当前任务相关的章节**：

- [`reference/code-examples.md`](reference/code-examples.md) — **代码手册（推荐首选）**：可运行片段与常见任务
- [`reference/architecture.md`](reference/architecture.md) — 核心 trait、Agent 构建器状态机、设计原则
- [`reference/dependencies-and-features.md`](reference/dependencies-and-features.md) — `rig` / `rig-core`、feature、Tokio、TLS
- [`reference/providers-and-clients.md`](reference/providers-and-clients.md) — 内置 Provider 列表、客户端初始化模式
- [`reference/agents-completion-streaming.md`](reference/agents-completion-streaming.md) — `Prompt` / `Chat` / 结构化输出、流式、请求钩子、记忆
- [`reference/vector-stores-and-rag.md`](reference/vector-stores-and-rag.md) — `VectorStoreIndex`、`top_n` / `top_n_ids`、RAG 流程
- [`reference/tools-and-macros.md`](reference/tools-and-macros.md) — `Tool` trait、`#[rig_tool]`、`ToolSet`、MCP（`rmcp` feature）
- [`reference/wasm-and-errors.md`](reference/wasm-and-errors.md) — WASM 边界、错误与 `ProviderClientError`
- [`reference/engineering-constraints.md`](reference/engineering-constraints.md) — 在本仓库内改 Rig 时的约定摘要

**说明**：可运行示例以 **[`reference/code-examples.md`](reference/code-examples.md)** 为主；若你本地检出了完整 Rig 源码树，可在该树内自行浏览其自带的 example / 集成测试目录（本 Skill 不链接到树外路径）。

## 验证命令（修改库代码时）

```bash
cargo fmt
cargo clippy --all-targets --all-features
cargo test
```

纯文档或 Skill 变更若无法跑全量测试，应说明未执行项。

## 常见陷阱（Code Agent 自检）

1. **路径与 feature**：配套向量库在未启用对应 feature 时**不存在**于 `rig::` 下；应对照依赖 manifest 与门面 crate 中 `cfg(feature = "...")` 门控（详见 [`reference/dependencies-and-features.md`](reference/dependencies-and-features.md)）。
2. **模型常量**：模型 ID 以各 `providers::<name>` 模块中**实际导出常量**为准（如 `openai::GPT_5_2`），勿复制旧博客中的已废弃名称。
3. **derive**：`Embed` / `rig_tool` 需要启用 **`derive`** feature。
4. **WASM**：避免在公共 trait 边界写死 `Send + Sync`；使用 `WasmCompatSend` / `WasmCompatSync`（见 [`reference/wasm-and-errors.md`](reference/wasm-and-errors.md)）。
5. **工具与向量**：实现 `VectorStoreIndex` 的类型会自动获得 `Tool` 能力，可用于 Agent 检索；与手写工具并存时注意 `tool_choice` 与 `ToolChoice` 语义。
