# Rig 代码示例手册（可复制）

以下片段与仓库内 `examples/`、`tests/providers/` 用法一致；模型常量、feature 名随版本变化时，以当前 crate 源码与 `Cargo.toml` 为准。

---

## 1. 最小可运行二进制（单次 `prompt`）

**`Cargo.toml`**

```toml
[package]
name = "rig-demo"
version = "0.1.0"
edition = "2024"

[dependencies]
rig = "0.36.0"
tokio = { version = "1", features = ["full"] }
anyhow = "1"
```

**`src/main.rs`**（需环境变量，例如 OpenAI：`OPENAI_API_KEY`）

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

    let answer = agent.prompt("What is Rig in one sentence?").await?;
    println!("{answer}");
    Ok(())
}
```

---

## 2. 使用 `rig-core`（不通过根门面 `rig`）

依赖：

```toml
rig-core = "0.36.0"
tokio = { version = "1", features = ["full"] }
```

```rust
use rig_core::client::{CompletionClient, ProviderClient};
use rig_core::completion::Prompt;
use rig_core::providers::openai;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = openai::Client::from_env()?;
    let agent = client.agent(openai::GPT_4O).build();
    println!("{}", agent.prompt("Hi").await?);
    Ok(())
}
```

---

## 3. 本地 Ollama（无 API key 时常用）

```rust
use rig::client::{CompletionClient, Nothing, ProviderClient};
use rig::completion::Prompt;
use rig::providers::ollama;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 默认 http://localhost:11434；也可用 Client::from_env() 或 builder 配 base_url / api_key
    let client = ollama::Client::new(Nothing)?;

    let agent = client
        .agent("llama3.2")
        .preamble("Be brief.")
        .temperature(0.3)
        .build();

    println!("{}", agent.prompt("Explain Rust ownership in 3 bullets.").await?);
    Ok(())
}
```

嵌入（维度需与模型一致）：

```rust
use rig::client::{EmbeddingsClient, Nothing};
use rig::providers::ollama;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = ollama::Client::new(Nothing)?;
    let model = client.embedding_model("all-minilm", 384);
    let vectors = model.embed_texts(vec!["hello".into(), "world".into()]).await?;
    println!("{} vectors", vectors.len());
    Ok(())
}
```

---

## 4. `AgentBuilder` 显式构建（等价于 `client.agent(...)`）

```rust
use rig::agent::AgentBuilder;
use rig::client::{CompletionClient, ProviderClient};
use rig::completion::Prompt;
use rig::providers::openai;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let openai = openai::Client::from_env()?;
    let model = openai.completion_model(openai::GPT_4O);

    let agent = AgentBuilder::new(model)
        .preamble("You are Gandalf.")
        .temperature(0.8)
        .max_tokens(500)
        .build();

    println!("{}", agent.prompt("Speak.").await?);
    Ok(())
}
```

---

## 5. RAG：嵌入 + 内存向量索引 + `dynamic_context`

**`Cargo.toml`** 需启用宏：

```toml
rig = { version = "0.36.0", features = ["derive"] }
serde = { version = "1", features = ["derive"] }
tokio = { version = "1", features = ["full"] }
anyhow = "1"
```

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
struct Chunk {
    id: String,
    title: String,
    #[embed]
    body: String,
}

#[tokio::main]
async fn main() -> Result<()> {
    let openai = openai::Client::from_env()?;
    let embedding_model = openai.embedding_model(openai::TEXT_EMBEDDING_3_SMALL);

    let embeddings = EmbeddingsBuilder::new(embedding_model.clone())
        .documents(vec![Chunk {
            id: "1".into(),
            title: "Rig".into(),
            body: "Rig is a Rust library for LLM applications.".into(),
        }])?
        .build()
        .await?;

    let store = InMemoryVectorStore::from_documents(embeddings);
    let index = store.index(embedding_model);

    let agent = openai
        .agent(openai::GPT_4O)
        .preamble("Use the retrieved context when relevant. If missing, say you don't know.")
        .dynamic_context(2, index)
        .build();

    println!("{}", agent.prompt("What is Rig?").await?);
    Ok(())
}
```

要点：`#[derive(Embed)]` + `#[embed]` 标出参与向量化的字段；`dynamic_context(k, index)` 每轮自动检索 `k` 条。

---

## 6. 直接调用 `VectorStoreIndex::top_n`（不经过 Agent）

下面示例在 **同一 `main`** 中建库并调用 `top_n`（需 `features = ["derive"]` 与第 5 节相同的 `Chunk` 定义亦可复用）：

```rust
use anyhow::Result;
use rig::client::{EmbeddingsClient, ProviderClient};
use rig::embeddings::EmbeddingsBuilder;
use rig::providers::openai;
use rig::vector_store::in_memory_store::InMemoryVectorStore;
use rig::vector_store::{VectorSearchRequest, VectorStoreIndex};
use rig::Embed;
use serde::Serialize;

#[derive(Embed, Serialize, Clone, Debug, Eq, PartialEq, Default)]
struct Chunk {
    id: String,
    #[embed]
    body: String,
}

#[tokio::main]
async fn main() -> Result<()> {
    let client = openai::Client::from_env()?;
    let emb = client.embedding_model(openai::TEXT_EMBEDDING_3_SMALL);

    let embeddings = EmbeddingsBuilder::new(emb.clone())
        .documents(vec![Chunk {
            id: "1".into(),
            body: "Rig is a Rust LLM library.".into(),
        }])?
        .build()
        .await?;

    let index = InMemoryVectorStore::from_documents(embeddings).index(emb);

    let req = VectorSearchRequest::builder()
        .query("rig library")
        .samples(3)
        .build();

    let hits: Vec<(f64, String, serde_json::Value)> = index.top_n(req).await?;
    for (score, id, doc) in hits {
        println!("{score} {id} {doc}");
    }
    Ok(())
}
```

内存索引的 `Filter` 默认为 `Filter<serde_json::Value>`；其他后端会替换为各自的过滤类型。

---

## 7. 多轮对话：`Chat` + 自建 `Vec<Message>`

```rust
use rig::client::{CompletionClient, ProviderClient};
use rig::completion::Chat;
use rig::message::Message;
use rig::providers::openai;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let agent = openai::Client::from_env()?.agent(openai::GPT_4O).build();

    let mut history: Vec<Message> = Vec::new();

    let a = agent.chat("My name is Bob.", &mut history).await?;
    println!("turn1: {a}");

    let b = agent.chat("What's my name?", &mut history).await?;
    println!("turn2: {b}");
    Ok(())
}
```

---

## 8. 结构化输出：`TypedPrompt` + `JsonSchema`

```rust
use rig::client::{CompletionClient, ProviderClient};
use rig::completion::TypedPrompt;
use rig::providers::openai;
use schemars::JsonSchema;
use serde::Deserialize;

#[derive(Debug, Deserialize, JsonSchema)]
struct Summary {
    title: String,
    bullets: Vec<String>,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let agent = openai::Client::from_env()?.agent(openai::GPT_4O).build();

    let out: Summary = agent
        .prompt_typed("Summarize Rust ownership in a title and 3 bullets.")
        .await?;

    println!("{out:?}");
    Ok(())
}
```

依赖需包含 `schemars`（版本与 `rig` 的 workspace 对齐，例如 `1.0` 系）。

---

## 9. 会话记忆：`InMemoryConversationMemory` + `.conversation(id)`

与仓库 `examples/agent_with_memory.rs` 一致：

```rust
use anyhow::Result;
use rig::client::{CompletionClient, ProviderClient};
use rig::completion::Prompt;
use rig::memory::InMemoryConversationMemory;
use rig::providers::openai;

#[tokio::main]
async fn main() -> Result<()> {
    let memory = InMemoryConversationMemory::new();

    let agent = openai::Client::from_env()?
        .agent(openai::GPT_4O)
        .preamble("You are a helpful assistant with memory.")
        .memory(memory)
        .build();

    let _ = agent.prompt("My name is Alice.").conversation("user-123").await?;
    let reply = agent
        .prompt("What's my name?")
        .conversation("user-123")
        .await?;

    println!("{reply}");
    Ok(())
}
```

高级策略（滑动窗口、token 预算等）在配套 crate **`rig-memory`**（根 `rig` 的 `memory` feature）。

---

## 10. 请求钩子：`PromptHook` + `.with_hook`

摘自 `examples/request_hook.rs` 的精简版：

```rust
use rig::agent::{HookAction, PromptHook};
use rig::client::{CompletionClient, ProviderClient};
use rig::completion::{CompletionModel, CompletionResponse, Message, Prompt};
use rig::message::UserContent;
use rig::providers::openai;

#[derive(Clone)]
struct LogHook;

impl<M: CompletionModel> PromptHook<M> for LogHook {
    async fn on_completion_call(&self, prompt: &Message, _history: &[Message]) -> HookAction {
        if let Message::User { content } = prompt {
            let text: String = content
                .iter()
                .filter_map(|c| match c {
                    UserContent::Text(t) => Some(t.text.clone()),
                    _ => None,
                })
                .collect::<Vec<_>>()
                .join("\n");
            println!("→ sending: {text}");
        }
        HookAction::cont()
    }

    async fn on_completion_response(
        &self,
        _prompt: &Message,
        response: &CompletionResponse<M::Response>,
    ) -> HookAction {
        println!("← choice: {:?}", response.choice);
        HookAction::cont()
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let agent = openai::Client::from_env()?.agent(openai::GPT_4O).build();

    let out = agent
        .prompt("Say hi in 5 words.")
        .with_hook(LogHook)
        .await?;
    println!("{out}");
    Ok(())
}
```

---

## 11. 流式：`StreamingPrompt` / `StreamingChat`

与 `examples/agent_stream_chat.rs` 一致的模式：`stream_*().await` 得到流，再用 `futures::StreamExt` 消费 `MultiTurnStreamItem`。

```rust
use anyhow::{Result, anyhow};
use futures::StreamExt;
use rig::agent::{MultiTurnStreamItem, StreamingResult};
use rig::client::{CompletionClient, ProviderClient};
use rig::message::Message;
use rig::providers::openai;
use rig::streaming::StreamingChat;

async fn collect_final<R>(stream: &mut StreamingResult<R>) -> Result<String> {
    let mut last = None;
    while let Some(item) = stream.next().await {
        if let MultiTurnStreamItem::FinalResponse(r) = item? {
            last = Some(r.response().to_owned());
        }
    }
    last.ok_or_else(|| anyhow!("no final response"))
}

#[tokio::main]
async fn main() -> Result<()> {
    let agent = openai::Client::from_env()?.agent(openai::GPT_4).build();

    let history = vec![
        Message::user("Tell me a joke."),
        Message::assistant("Why did the chicken cross the road?".into()),
    ];

    let mut stream = agent.stream_chat("Continue the joke.", &history).await;
    let text = collect_final(&mut stream).await?;
    println!("{text}");
    Ok(())
}
```

单轮流式用 `rig::streaming::StreamingPrompt` 的 `stream_prompt`。

---

## 12. 工具：`#[tool_macro]`（需 `derive` feature）

根 crate 将 `rig_derive::rig_tool` 重导出为 **`rig::tool_macro`**：

```rust
use rig::client::{CompletionClient, ProviderClient};
use rig::completion::Prompt;
use rig::providers::openai;
use rig::tool::ToolError;
use rig::tool_macro;

#[tool_macro(
    description = "Adds two integers",
    params(a = "First summand", b = "Second summand")
)]
fn add(a: i32, b: i32) -> Result<i32, ToolError> {
    Ok(a + b)
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let agent = openai::Client::from_env()?
        .agent(openai::GPT_4O)
        .tool(Add)
        .build();

    println!("{}", agent.prompt("What is 40 + 2? Use the tool.").await?);
    Ok(())
}
```

生成的工具类型名一般为函数名的 PascalCase（此处为 `Add`）。

---

## 13. 工具：手写 `Tool` trait（与 `manual_tool_calls` 一致）

```rust
use rig::completion::ToolDefinition;
use rig::tool::Tool;
use serde::{Deserialize, Serialize};
use serde_json::json;

#[derive(Deserialize)]
struct Args {
    x: i32,
    y: i32,
}

#[derive(Debug, thiserror::Error)]
#[error("math error")]
struct MathErr;

#[derive(Deserialize, Serialize)]
struct AddTool;

impl Tool for AddTool {
    const NAME: &'static str = "add";
    type Error = MathErr;
    type Args = Args;
    type Output = i32;

    async fn definition(&self, _prompt: String) -> ToolDefinition {
        ToolDefinition {
            name: Self::NAME.into(),
            description: "Add x and y".into(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "x": { "type": "integer" },
                    "y": { "type": "integer" }
                },
                "required": ["x", "y"]
            }),
        }
    }

    async fn call(&self, args: Args) -> Result<i32, MathErr> {
        Ok(args.x + args.y)
    }
}
```

注册：`client.agent(model).tool(AddTool).build()`。

---

## 14. 启用配套向量库（以 Qdrant 为例）

**`Cargo.toml`**

```toml
rig = { version = "0.36.0", features = ["derive", "qdrant"] }
```

应用代码从 **`rig::qdrant`** 导入（具体类型名以 `crates/rig-qdrant` 与 README 为准），连接 URL、集合名、维度需与部署一致。

---

## 15. 错误类型分层（`?` 传播时心里要有数）

```rust
use rig::client::ProviderClient;
use rig::providers::openai;

fn build_client() -> Result<openai::Client, rig::client::ProviderClientError> {
    openai::Client::from_env()
}
```

一次异步调用里常见对应关系：

- 仅 `from_env()` / builder：失败为 **`ProviderClientError`**
- `agent.prompt(...).await`：失败为 **`PromptError`**（含自动工具执行失败）
- `agent.prompt_typed(...).await`：失败为 **`StructuredOutputError`**
- `index.top_n(...).await` / 嵌入构建：失败为 **`VectorStoreError`** / **`EmbeddingError`**

---

## 16. WASM：`cfg` 区分错误类型（自定义后端时）

```rust
#[cfg(not(target_family = "wasm"))]
type DynErr = Box<dyn std::error::Error + Send + Sync + 'static>;

#[cfg(target_family = "wasm")]
type DynErr = Box<dyn std::error::Error + 'static>;
```

公开 trait 边界优先使用 Rig 已提供的 `WasmCompatSend` / `WasmCompatSync`，不要手写 `Send + Sync`。
