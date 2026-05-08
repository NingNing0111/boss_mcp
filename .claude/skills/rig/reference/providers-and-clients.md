# Provider 与 Client 使用说明

## 模块位置

所有**内置** HTTP Provider 位于：

`rig::providers::*`（或 `rig_core::providers::*`）

当前仓库 `rig-core/src/providers/mod.rs` 声明的模块包括（名称即子模块路径）：

`anthropic`、`azure`、`chatgpt`、`cohere`、`copilot`、`deepseek`、`galadriel`、`gemini`、`groq`、`huggingface`、`hyperbolic`、`llamafile`、`minimax`、`mira`、`mistral`、`moonshot`、`ollama`、`openai`、`openrouter`、`perplexity`、`together`、`voyageai`、`xai`、`xiaomimimo`、`zai`。

**新增或更名 Provider 时**，以该 `mod.rs` 为准，勿依赖本列表永久不变。

## 独立配套 crate（不在 `providers::*` 的「全家桶」里）

以下通过 **`rig` 根 crate 的 feature** 暴露为 `rig::bedrock`、`rig::vertexai`、`rig::gemini_grpc`、`rig::fastembed` 等（见根 `src/lib.rs`）：

- AWS Bedrock → `rig-bedrock`
- Google Vertex AI → `rig-vertexai`
- Gemini gRPC → `rig-gemini-grpc`
- Fastembed（本地嵌入）→ `rig-fastembed`

## 客户端初始化模式

1. **环境变量（最常见）**

   ```rust
   use rig::client::ProviderClient;
   use rig::providers::openai;

   let client = openai::Client::from_env()?;
   ```

   错误类型：`rig::client::ProviderClientError`（例如缺少 `OPENAI_API_KEY`）。

2. **显式配置**

   各 Provider 提供 `Client::builder()` 或 `from_val` 等（见具体模块文档）；**密钥与 base URL 以该 Provider 实现为准**。

3. **按能力获取模型**

   ```rust
   use rig::client::{CompletionClient, EmbeddingsClient};
   use rig::providers::openai;

   fn use_models(client: openai::Client) {
       let _completion = client.completion_model(openai::GPT_4O);
       let _embedding = client.embedding_model(openai::TEXT_EMBEDDING_3_SMALL);
   }
   ```

   若某 Client 未实现 `EmbeddingsClient`，则**没有** `embedding_model` 方法（编译期约束）。

4. **Ollama 本地（常见无密钥场景）** — 详见 `code-examples.md` 第 3 节；核心为 `Client::new(Nothing)` 或 `Client::builder()`：

   ```rust
   use rig::client::{CompletionClient, Nothing};
   use rig::completion::Prompt;
   use rig::providers::ollama;

   #[tokio::main]
   async fn main() -> Result<(), Box<dyn std::error::Error>> {
       let client = ollama::Client::new(Nothing)?;
       let agent = client.agent("llama3.2").build();
       println!("{}", agent.prompt("Ping.").await?);
       Ok(())
   }
   ```

## Agent 快捷入口

对实现了 `CompletionClient` 的 Client：

```rust
use rig::client::{CompletionClient, ProviderClient};
use rig::completion::Prompt;
use rig::providers::openai;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = openai::Client::from_env()?;
    let agent = client
        .agent(openai::GPT_4O)
        .preamble("You are a helpful assistant.")
        .build();

    let text = agent.prompt("Hello").await?;
    println!("{text}");
    Ok(())
}
```

## 模型常量

每个 Provider 在模块内导出模型 ID 常量（如 `openai::GPT_4O`）。**务必使用当前版本 crate 中实际存在的符号**；不同版本可能增删模型常量。

## 与其他主题的交叉引用

- 结构化输出、多轮、流式：见 `agents-completion-streaming.md`
- Bedrock / Vertex 专用类型：启用 feature 后阅读对应 `crates/rig-bedrock`、`crates/rig-vertexai` 的 README 与示例

## 实现自定义 Provider（高级）

新 Provider 应贴近现有实现（`AGENTS.md` 建议 OpenAI 兼容实现参考 `crates/rig-core/src/providers/openai/`），包含：

- Extension / Builder、`Provider` 实现、`Capabilities`、`ProviderBuilder`
- 请求：Rig `CompletionRequest` → 厂商 body；响应 → `CompletionResponse`
- 若支持：流式、遥测 span（GenAI 语义约定）、测试或示例

勿添加真实 API 中不存在的字段。
