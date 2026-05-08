# 依赖、Feature 与 crate 选择

## `rig` 与 `rig-core`

- **`rig`（根门面）**  
  - `pub use rig_core::*;`  
  - 通过 **optional 依赖 + `#[cfg(feature = ...)]`** 再导出 `rig::lancedb`、`rig::bedrock` 等子模块。  
  - 适合：一个 `Cargo.toml` 里按需打开多种向量库或 Bedrock / Vertex 等。

- **`rig-core`**  
  - 核心 trait、内置 Provider（`providers::*`）、Agent、向量抽象、内存向量库等。  
  - 适合：希望依赖树更小、不需要任何 `rig-*` 配套 crate 的场景。

**以仓库根目录 [`Cargo.toml`](../../../Cargo.toml) 为准**核对版本号与 feature 名称；对外文档可能滞后于 main 分支。

## 根 crate `rig` 常见 feature（节选 — 以源码为准）

以下名称来自当前仓库配置，用于 Agent 写 `Cargo.toml` 时对齐；**若冲突以 `Cargo.toml` `[features]` 为真**。

| Feature | 作用 |
|---------|------|
| `default` | 通常含 `rig_core/default` 与 `rustls` |
| `all` | 转发 `rig_core/all`（如 derive、pdf、rayon 等组合，见 rig-core） |
| `derive` | `rig_core/derive` → `Embed` 与 `rig_tool`（`tool_macro`） |
| `bedrock` / `vertexai` / `gemini-grpc` / `fastembed` 等 | 启用对应 `rig-*` 子模块 |
| `lancedb` / `qdrant` / `mongodb` / `sqlite` / `postgres` / … | 启用对应向量或存储集成 |
| `memory` | `rig-memory` 与 `rig::memory` 扩展导出 |
| `audio` / `image` / `pdf` / `epub` | 转发 `rig_core` 中媒体与文档能力 |
| `rmcp` | MCP 工具客户端等 |
| `wasm` | `rig_core/wasm` |
| `rustls` / `native-tls` | TLS 策略（并传递到部分子 crate） |
| `reqwest-middleware` 等 | HTTP 中间件栈 |

**fastembed** 在根 crate 中可能拆成 `fastembed`、`fastembed-hf-hub`、`fastembed-ort-download-binaries` 等细粒度 feature，按是否需要 HF Hub 或 ORT 二进制选择。

## `rig-core` 自身 feature（节选）

- `derive`：`rig-derive`（`Embed`、`rig_tool`）。
- `audio` / `image`：声明能力，配合具体 Provider 实现。
- `pdf` / `epub`：文档加载相关依赖。
- `discord-bot`：Discord 集成。
- `rmcp`：MCP。
- `wasm`：WASM 兼容依赖与随机源等。
- `rustls` / `native-tls`、`websocket-*`、`socks`、`reqwest-middleware-*`：网络栈细节。

## 异步运行时

示例与库本身普遍假设 **`tokio`**。使用 `#[tokio::main]` 时需启用 tokio 的 `macros` 与 `rt-multi-thread`（或 `full`），否则编译失败。

## 在 Agent 代码中的推荐写法

**仅对话 + OpenAI（默认 TLS）**

```toml
[dependencies]
rig = "0.36.0"
tokio = { version = "1", features = ["full"] }
anyhow = "1"
```

**RAG（需要 `Embed` 派生）**

```toml
[dependencies]
rig = { version = "0.36.0", features = ["derive"] }
serde = { version = "1", features = ["derive"] }
tokio = { version = "1", features = ["full"] }
anyhow = "1"
```

**对话 + 工具宏 + Qdrant**

```toml
[dependencies]
rig = { version = "0.36.0", features = ["derive", "qdrant"] }
tokio = { version = "1", features = ["full"] }
anyhow = "1"
```

**结构化输出**（`prompt_typed`）还需与 `rig` 对齐的 `schemars`：

```toml
schemars = "1.0"
```

版本号应替换为当前项目使用的 crates.io 版本；**路径依赖**本地 fork 时使用 `path = "..."` 并省略或调整 version。

**二进制入口模板**

```rust
#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // rig::...
    Ok(())
}
```

## 文档与 docs.rs

- docs.rs 上 `rig` / `rig-core` 通常带 **`all-features`**，可看到 cfg 门控模块；本地阅读可使用：

  `cargo doc --workspace --no-deps --open`
