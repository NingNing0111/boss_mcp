# Rig 架构与核心抽象

## 设计目标

Rig 用**少量 Provider 无关的 trait** 覆盖「补全 / 嵌入 / 向量检索 / 工具」，上层以 **Agent** 与 **Builder** 组合这些能力，减少在每个模型 API 上重复造轮子。

## 四大核心 trait（优先复用，勿平行造新抽象）

| Trait | 职责 | 典型实现方 |
|--------|------|------------|
| `completion::CompletionModel` | 将 Rig 的 `CompletionRequest` 转为厂商请求并解析为 `CompletionResponse` | 各 Provider 的 `*Model` 类型 |
| `embeddings::EmbeddingModel` | 文本 → `Embedding` | 各 Provider 的 embedding 模型句柄 |
| `vector_store::VectorStoreIndex` | 相似度检索：`top_n`、`top_n_ids`，带后端相关 `Filter` | 内存索引、`rig-qdrant` 等 |
| `tool::Tool` | JSON 参数进出、工具元数据（`ToolDefinition`） | 手写 struct、`#[rig_tool]` 生成、向量索引的 blanket 实现 |

向量写入侧常见配套 trait：`vector_store::InsertDocuments`（将文档 + 已计算嵌入写入存储）。

## Agent 与 AgentBuilder

- **`Agent<M>`**：持有完成模型 `M: CompletionModel` 与运行时配置（preamble、静态/动态上下文、工具集、可选 memory、结构化 schema 等）。
- **`AgentBuilder<M, P, ToolState>`** 使用 **typestate**：
  - 工具配置**互斥**：要么通过 `.tool()` / `.tools()` 等逐步添加（最终构建 `ToolServer`），要么传入已有 `ToolServerHandle`，二者不可混用。
  - `PromptHook` 泛型 `P` 用于每请求生命周期钩子（可 `Continue` / `Terminate`；工具调用侧还有 `Skip` 等）。修改钩子逻辑时需同时考虑流式与非流式路径（见仓库 `AGENTS.md`）。

常用链式调用与根目录文档一致，例如：

```rust
use anyhow::Result;
use rig::agent::AgentBuilder;
use rig::client::{CompletionClient, ProviderClient};
use rig::completion::Prompt;
use rig::providers::openai;

#[tokio::main]
async fn main() -> Result<()> {
    let client = openai::Client::from_env()?;

    // 方式 A：client 上的快捷方法
    let agent = client
        .agent(openai::GPT_4O)
        .preamble("You are helpful.")
        .temperature(0.8)
        .build();

    // 方式 B：显式 AgentBuilder（等价）
    let model = client.completion_model(openai::GPT_4O);
    let same = AgentBuilder::new(model)
        .preamble("You are helpful.")
        .temperature(0.8)
        .build();

    println!("{}", agent.prompt("Hi").await?);
    println!("{}", same.prompt("Hi").await?);
    Ok(())
}
```

## 客户端泛型架构

Provider 的 `Client` 多为泛型结构（扩展能力 + HTTP 后端），并通过 `Capable<T>` 之类机制声明「是否支持补全 / 嵌入 / 图片」等。**只有实现了对应 client trait 的 Client** 才能调用 `completion_model`、`embedding_model` 等方法。

初始化首选 **`ProviderClient::from_env()`**（或 `from_val`），错误类型为 **`ProviderClientError`**（缺密钥、环境变量非法等），与请求阶段错误区分。

## 类型擦除与动态上下文

- `vector_store::VectorStoreIndexDyn`：用于 `AgentBuilder::dynamic_context(sample, impl ...)` 等需要 `Arc<dyn ...>` 的场景。
- Agent 内部将向量索引与工具、memory 等一并编排进多轮补全循环。

## 与 AGENTS.md 对齐的要点

- 可配置公开类型遵循 **Builder** 风格。
- 新错误用 **`thiserror`** 枚举，避免 `String` 作为错误类型。
- 向量存储配套实现须同时实现 **`top_n` 与 `top_n_ids`**，过滤器用后端的 `Filter` 类型，错误返回 **`VectorStoreError`**。

## 延伸阅读

- 依赖与 feature：`dependencies-and-features.md`
- Agent 层补全与流式：`agents-completion-streaming.md`
