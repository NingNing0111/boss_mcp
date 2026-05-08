# 向量存储、索引与 RAG

## 核心 trait（`rig::vector_store`）

- **`VectorStoreIndex`**  
  - `type Filter: SearchFilter`：后端特定过滤语法。  
  - **`top_n`**：返回 `(score, id, document)`，文档反序列化为调用方指定的 `T: Deserialize`。  
  - **`top_n_ids`**：仅返回 id + 分数（轻量查询）。  

- **`InsertDocuments`**：批量插入「文档 + 嵌入向量」，用于构建库。

- **`VectorSearchRequest`**：构造查询（文本、top k、过滤器等），见 `vector_store::request`。

- **`VectorStoreError`**：统一错误面；嵌入失败包装为 `EmbeddingError`，HTTP 类为 `ReqwestError` / `ExternalAPIError` 等。

实现新后端时：**同时实现 `top_n` 与 `top_n_ids`**，错误勿用裸 `String`（`AGENTS.md`）。

## 与 Agent 的衔接

- 任意实现 `VectorStoreIndex` 且满足 `AgentBuilder::dynamic_context` 约束的类型，可作为 **动态上下文** 注入 Agent。
- `VectorStoreIndex` 的实现者还会获得 **`Tool`** 的 blanket 实现，使模型可通过「工具调用」做检索（与 `dynamic_context` 注入是两种互补用法，按产品需求选择或组合）。

## 嵌入与文档类型

1. 在数据结构上 **`#[derive(Embed, Serialize, ...)]`**（需 **`derive`** feature），并用 **`#[embed]`** 标记参与向量化的字段（见 `examples/rag.rs`）。
2. 使用 **`EmbeddingsBuilder::new(embedding_model)`** 加载文档、调用 **`build().await?`** 得到嵌入结果列表。
3. 将嵌入交给具体存储：例如内存 **`InMemoryVectorStore::from_documents(embeddings)`**，再 **`vector_store.index(embedding_model)`** 得到可查询的 **index**。

**注意**：索引在查询时通常仍需 **同一套 `EmbeddingModel`**（或维度一致的模型），以保证向量空间一致。

### 代码骨架（与 `examples/rag.rs` 一致）

```rust
use anyhow::Result;
use rig::client::{CompletionClient, EmbeddingsClient, ProviderClient};
use rig::completion::Prompt;
use rig::embeddings::EmbeddingsBuilder;
use rig::providers::openai;
use rig::vector_store::in_memory_store::InMemoryVectorStore;
use rig::Embed;
use serde::Serialize;

#[derive(Embed, Serialize, Clone, Debug, Eq, PartialEq, Default)]
struct Doc {
    id: String,
    #[embed]
    text: String,
}

#[tokio::main]
async fn main() -> Result<()> {
    let client = openai::Client::from_env()?;
    let emb = client.embedding_model(openai::TEXT_EMBEDDING_3_SMALL);

    let embeddings = EmbeddingsBuilder::new(emb.clone())
        .documents(vec![Doc {
            id: "1".into(),
            text: "Rig is a Rust LLM library.".into(),
        }])?
        .build()
        .await?;

    let index = InMemoryVectorStore::from_documents(embeddings).index(emb);

    let agent = client
        .agent(openai::GPT_4O)
        .dynamic_context(2, index)
        .preamble("Answer using context when present.")
        .build();

    println!("{}", agent.prompt("What is Rig?").await?);
    Ok(())
}
```

更长的分节示例（含 `top_n`、错误类型）见 **`code-examples.md`**。

## 配套 crate 与 `rig` feature 映射

向量与部分存储在独立 crate 中；通过根 **`rig` 的 feature** 启用后，从 **`rig::mongodb`**、`rig::lancedb`、`rig::neo4j`、`rig::qdrant`、`rig::sqlite`、`rig::surrealdb`、`rig::milvus`、`rig::scylladb`、`rig::s3vectors`、`rig::helixdb`、`rig::vectorize`、`rig::postgres` 等入口访问（**以根 `src/lib.rs` 为准**）。

各 crate 的 README 在 `crates/rig-<name>/README.md`，含连接串、过滤类型与示例。

## 自定义向量存储

若内置后端不满足需求：

1. 在新 crate 或应用模块中实现 **`VectorStoreIndex` + `InsertDocuments`（如需要写入）**。  
2. 使用后端原生过滤器实现 **`SearchFilter`**。  
3. 在 Agent 侧通过 **`dynamic_context`** 或 **`Tool`** 暴露给模型。  

参考：`examples/custom_vector_store.rs`。

## RAG 流程清单（Code Agent 实施步骤）

1. 定义带 `#[embed]` 的文档类型；确认 `Serialize` 字段与 id 策略。  
2. 选择 `EmbeddingModel`（与数据语种、维度、成本匹配）。  
3. `EmbeddingsBuilder` 批量嵌入 → 写入存储 → 构建 `index`。  
4. `client.agent(...).dynamic_context(k, index).preamble(...)` 明确指示模型如何使用检索片段。  
5. 评估是否需要 **`tool_choice`**、引用格式、以及多轮 `Chat` 历史。  

## 集成测试

仓库级集成测试在 `tests/integrations/` 等目录；运行前需本地服务或 testcontainers（见各测试模块说明）。
