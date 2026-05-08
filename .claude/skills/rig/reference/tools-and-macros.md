# 工具（Tools）、宏与 MCP

## `Tool` trait（`rig::tool`）

手写工具时实现 `Tool`：

- **`const NAME`**：工具名，与 JSON schema 一致。  
- **`type Args`**：`serde::Deserialize` 的参数结构体。  
- **`type Output`**：返回值类型（序列化后回传给模型）。  
- **`type Error`**：工具执行错误（勿滥用字符串）。  
- **`definition`**：异步生成 `completion::ToolDefinition`（`name`、`description`、`parameters` JSON Schema）。  
- **`call`**：执行逻辑。

将多个工具放入 **`ToolSet`**，或在 AgentBuilder 上 **`.tool(...)`** 注册。

实现 `Tool` 的类型可直接链式注册（此处略去 `impl Tool`，见下文宏示例与 `code-examples.md` 第 13 节）：

```rust
// client.agent(openai::GPT_4O).tool(MyTool).build()
```

完整手动编排 tool 循环（不用 `Prompt` 自动执行）：见 **`examples/manual_tool_calls.rs`**。

## `#[rig_tool]` 过程宏

- 来源：`rig_derive::rig_tool`；通过 `rig` / `rig_core` 的 **`derive`** feature 可使用 `rig::tool_macro` 重导出（即 `rig_core` 中的 `pub use rig_derive::rig_tool as tool_macro`）。
- 标注在**函数**上，函数签名需返回 **`Result<T, E>`**（宏会解析 `T` 与 `E`）。
- 支持 **`async fn`**。
- 常用参数：
  - `name = "..."`：显式工具名（长度与字符集限制见宏文档：字母/`_` 开头，可含数字、`_`、`-`，最长 64）。
  - `description = "..."`。
  - `params(foo = "...", bar = "...")`：参数说明。
  - `required(a, b, ...)`：必填参数列表。

生成的类型默认以函数名 PascalCase 命名，并实现 `Tool`。

启用 `derive` 后可用属性名 **`tool_macro`**（即 `rig_tool` 的重导出）：

```rust
use anyhow::Result;
use rig::client::{CompletionClient, ProviderClient};
use rig::completion::Prompt;
use rig::providers::openai;
use rig::tool::ToolError;
use rig::tool_macro;

#[tool_macro(description = "Multiply two integers")]
fn mul(a: i32, b: i32) -> Result<i32, ToolError> {
    Ok(a * b)
}

#[tokio::main]
async fn main() -> Result<()> {
    let agent = openai::Client::from_env()?
        .agent(openai::GPT_4O)
        .tool(Mul)
        .build();
    println!("{}", agent.prompt("What is 6 * 7? Use the tool.").await?);
    Ok(())
}
```

（生成类型名为 `Mul`，对应函数 `mul`。）更多见 **`code-examples.md` 第 12–13 节**。

## 与 `Prompt` 的关系

使用 **`agent.prompt(...)`** 时，若模型发起 tool call，Rig **自动**调度到已注册工具并继续对话直到文本回复。需要可见中间步骤时改用 **`Completion` + 手动循环**。

## Provider 托管工具

`completion::ProviderToolDefinition` 用于部分厂商的「内置工具」（如联网搜索类），通过请求里的 `additional_params.tools` 等路径传递（具体支持度因 Provider 而异）。集成时阅读目标 Provider 的 `completion` 实现。

## MCP（`rmcp` feature）

启用 **`rig` / `rig_core` 的 `rmcp`** 后，可使用 `rig_core::tool::rmcp` 等与 MCP 协议交互的能力（例如将远程 MCP 工具桥接到 `Agent`）。示例见根 `Cargo.toml` 中 **`[[example]] name = "rmcp"`**（`required-features = ["rmcp"]`）及 `crates/rig-core` 相关示例。

## 向量检索即工具

实现 **`VectorStoreIndex`** 的类型自动具备作为 **Tool** 的能力，便于模型按需在对话中发起检索；与 **`dynamic_context`** 自动注入片段的取舍：

| 方式 | 特点 |
|------|------|
| `dynamic_context` | 每轮自动检索固定条数注入，调用模型无感 |
| 作为 `Tool` | 模型显式决定何时查、查什么 |

## 调试建议

- 工具名 / 参数 schema 与模型所见必须一致；改名后检查是否需同步迁移历史会话。  
- 对强类型参数使用清晰 `description` 字段，减少模型胡填。  
