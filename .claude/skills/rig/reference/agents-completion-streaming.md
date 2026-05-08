# Agent、Completion 高层 API、流式与记忆

## 高层 trait（`rig::completion`）

由 `Agent` 等类型实现，是大多数应用应优先使用的接口：

| Trait | 行为概要 |
|--------|----------|
| **`Prompt`** | 单次用户消息 → 返回 `String`。若模型返回 **tool call**，Rig 会**自动执行工具**并将结果继续走对话直到得到文本；工具不存在或失败则返回 `PromptError`。 |
| **`Chat`** | 在已有 `Vec<Message>` 历史上追加用户消息并跑完本轮（含自动工具）；历史由调用方维护。 |
| **`TypedPrompt`** | 基于 `schemars::JsonSchema` + `serde` 的结构化输出；厂商支持原生 structured output 时会约束 JSON。返回 `StructuredOutputError` 等。 |
| **`Completion`** | 更低层、可逐请求覆盖参数（与 `manual_tool_calls` 示例配合时可完全自控工具循环）。 |

**与 `Prompt` 的对比**：若需要**手动**执行工具、观察每一步 tool call，使用 `Agent::completion` + 自行解析 `AssistantContent`（见仓库 `examples/manual_tool_calls.rs`）。

### 代码：`Prompt` / `Chat` / `TypedPrompt`

```rust
use anyhow::Result;
use rig::client::{CompletionClient, ProviderClient};
use rig::completion::{Chat, Prompt, TypedPrompt};
use rig::message::Message;
use rig::providers::openai;
use schemars::JsonSchema;
use serde::Deserialize;

#[derive(Debug, Deserialize, JsonSchema)]
struct Answer {
    text: String,
}

#[tokio::main]
async fn main() -> Result<()> {
    let agent = openai::Client::from_env()?.agent(openai::GPT_4O).build();

    let s = agent.prompt("Hello").await?;
    println!("{s}");

    let mut hist = Vec::<Message>::new();
    let s2 = agent.chat("Follow-up", &mut hist).await?;
    println!("{s2}");

    let a: Answer = agent
        .prompt_typed("Reply with one short sentence in field \"text\".")
        .await?;
    println!("{:?}", a);
    Ok(())
}
```

## AgentBuilder 常用配置

- **`preamble` / `append_preamble` / `without_preamble`**：系统提示。
- **`context`**：静态 `Document`，每次请求附带。
- **`dynamic_context(sample, index)`**：每轮从 `VectorStoreIndex` 采样 `sample` 条检索结果注入上下文（RAG 核心）。
- **`temperature` / `max_tokens` / `additional_params`**：采样与厂商扩展字段。
- **`tool` / `tools` / `dynamic_tools`**：注册工具（与 typestate 互斥规则见 `architecture.md`）。
- **`tool_choice`**：`message::ToolChoice`，如强制先调用工具等。
- **`default_max_turns`**：多轮 agent/tool 循环默认最大深度。
- **`memory(...)`**：实现 `memory::ConversationMemory` 的后端；请求侧通过 `PromptRequest::conversation` 指定会话 id（见 crate 文档字符串与 `examples/agent_with_memory.rs`）。
- **结构化输出**：`output_schema` / Builder 上对应方法（与 `TypedPrompt` 配合）。

## 请求生命周期钩子（`PromptHook`）

- 钩子可在发送前观察或修改请求，并返回 **`HookAction::Continue` 或 `Terminate`**。
- 工具阶段有 **`ToolCallHookAction`**：`Continue` / `Skip` / `Terminate`。
- 修改钩子后需覆盖 **流式与非流式** 两条代码路径（仓库 `AGENTS.md` 明确要求）。

## 流式输出

模块 **`rig::streaming`**：`Agent` 实现 `StreamingPrompt` / `StreamingChat`，典型用法是先 `.await` 得到 `StreamingResult`，再用 `futures::StreamExt` 迭代 `MultiTurnStreamItem`（完整可运行片段见 **`code-examples.md` 第 11 节** 与 `examples/agent_stream_chat.rs`）。

```rust
use anyhow::Result;
use futures::StreamExt;
use rig::agent::MultiTurnStreamItem;
use rig::client::{CompletionClient, ProviderClient};
use rig::providers::openai;
use rig::streaming::StreamingPrompt;

#[tokio::main]
async fn main() -> Result<()> {
    let agent = openai::Client::from_env()?.agent(openai::GPT_4O).build();
    let mut stream = agent.stream_prompt("Count from 1 to 3 slowly.").await;
    while let Some(item) = stream.next().await {
        match item? {
            MultiTurnStreamItem::FinalResponse(r) => {
                println!("done: {}", r.response());
            }
            _ => {}
        }
    }
    Ok(())
}
```

## 遥测

`rig::telemetry` 与 tracing span 对齐 **OpenTelemetry GenAI 语义约定**，便于在可观测性平台统一查看 span 属性。新增 Provider 时应跟随现有 span 命名与字段。

## 实用示例索引（仓库内）

- 简单对话：根 `README.md` 示例
- RAG：`examples/rag.rs`（需 `derive`）
- 请求钩子：`examples/request_hook.rs`
- 手动工具循环：`examples/manual_tool_calls.rs`
- 多 Agent / 辩论等：`examples/debate.rs`、`examples/agent_with_agent_tool.rs` 等

## 常见错误

- **`PromptError`**：工具失败、模型返回无法处理的内容等；区分于 `ProviderClientError`（建连/配置阶段）。
- **结构化输出**：目标类型须实现 `JsonSchema` + `DeserializeOwned`；厂商不支持时可能退化为提示词约束 JSON，鲁棒性因 Provider 而异。
