# WASM 兼容与错误处理

## 为何需要 `WasmCompatSend` / `WasmCompatSync`

Rig 在 **WASM** 目标上运行时，Rust 的 `Send` / `Sync` 约束过严会阻碍编译。核心公开 trait（如 **`Prompt`**、**`CompletionModel`**、**`VectorStoreIndex`** 等）使用 **`WasmCompatSend`** / **`WasmCompatSync`**（定义在 `rig::wasm_compat`），在原生平台别名到 `Send` / `Sync`，在 WASM 上放宽为 `()`。

**自定义类型实现这些 trait 时**，应沿用相同边界，而非写死 `Send + Sync`。

## `WasmBoxedFuture`

在需要 `Pin<Box<dyn Future>>` 的场景使用 **`WasmBoxedFuture`**，保证 WASM 与原生下 Future 约束一致（见 `vector_store` 等模块用法）。

## 错误类型中的 `Box<dyn Error>`

存储 `Box<dyn std::error::Error + ...>` 时按平台区分（与 `AGENTS.md` 一致）：

- **非 WASM**：`+ Send + Sync + 'static`  
- **WASM**：仅 `'static`（无 `Send + Sync`）

`VectorStoreError::DatastoreError` 等已按此模式处理；自定义后端应保持一致，避免 WASM 构建失败。

## 客户端与请求阶段错误

| 阶段 | 典型类型 | 说明 |
|------|-----------|------|
| 建客户端 / 读环境变量 | `ProviderClientError` | 缺密钥、URL 非法等 |
| 嵌入 / 补全 HTTP | 各 Provider 错误、`reqwest::Error` | 网络与状态码 |
| 高层提示词与工具 | `PromptError` | 工具不存在、执行失败、模型输出不符合预期 |
| 结构化输出 | `StructuredOutputError` | JSON 不匹配 schema 等 |
| 向量操作 | `VectorStoreError` | 嵌入、过滤、后端错误 |

新 API：**使用 `thiserror` 枚举**，避免 `String` 作为公共错误类型；不要用 `.unwrap()` / `.expect()` 处理可预期失败（仓库 clippy 默认禁止 `unwrap_used` 等）。

## 日志与诊断

项目普遍使用 **`tracing`**。在应用中初始化 `tracing_subscriber`（示例见 `examples/rag.rs`）以便观察 span 与 Rig 内部日志。

## 功能检测

若代码同时编译到 **native 与 `wasm32-unknown-unknown`**：

- 在 `Cargo.toml` 中为 WASM 关闭不可用依赖（或由 `rig` 的 `wasm` feature 处理）。  
- 避免在 WASM 中使用 **仅原生** 的 Provider 传输（如部分 WebSocket 路径）；以 `cfg(not(target_family = "wasm"))` 门控（参考 OpenAI Responses websocket 相关注释）。

```rust
#[cfg(not(target_family = "wasm"))]
fn only_native() {
    // 例如：依赖 tokio::net 或 OpenAI Responses 的 websocket 传输
}
```
