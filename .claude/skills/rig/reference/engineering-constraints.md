# 与本仓库协作时的工程约定（摘要）

面向在 **Rig 源码树内**改代码的 Agent：下列约定与仓库根目录贡献者文档一致，此处仅作 Skill 内自洽摘要（无外部文件链接）。

## 原则

- 修改前先读现有实现；改动范围贴合任务描述。
- 优先复用既有 trait：`CompletionModel`、`EmbeddingModel`、`VectorStoreIndex`、`Tool`，以及既有 Builder、模块划分与错误类型，避免平行抽象。
- 勿添加 TODO、占位 API 或未经验证的厂商字段。
- 新错误用 `thiserror` 枚举，避免用 `String` 作为公开错误类型；避免在可失败路径上随意 `.unwrap()` / `.expect()`。
- 文档与示例中的模型常量、feature 名、模块路径须与当前代码和 manifest 一致。

## WASM

- 在 trait 边界使用 `WasmCompatSend` / `WasmCompatSync`，而非写死 `Send` / `Sync`。
- 需要 boxed future 时使用 `WasmBoxedFuture`。
- 存储 `Box<dyn Error>` 时在非 WASM 与 WASM 目标上使用不同的 `Send + Sync` 约束（与核心库中 `VectorStoreError` 等模式一致）。

## Provider 与向量库

- 新 Provider 应对齐最接近的现有实现；向量存储配套须实现 `top_n` 与 `top_n_ids`，错误返回 `VectorStoreError`。
- Prompt / 工具钩子修改时须兼顾流式与非流式路径。

## 验证

- 在可行时运行 `cargo fmt`、`cargo clippy --all-targets --all-features`、`cargo test`；仅文档变更若无法跑全量测试应明确说明。
