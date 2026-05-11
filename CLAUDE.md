# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Build and Run Commands

```bash
# Build release
cargo build --release

# Run tests
cargo test

# Run a single test
cargo test <test_name>

# Run the MCP server
cargo run

# Run a standalone handler binary (for testing individual features)
cargo run --bin <binary_name>
```

Available standalone binaries:
- `login` — Boss 直聘 QR code login
- `login_check` — Boss 直聘 login check
- `qcc_login` — 企查查 QR code login
- `qcc_login_check` — 企查查 login check
- `search_position` — Job search test
- `position_detail` — Job detail test
- `start_chat` — Start chat with HR
- `get_unread_chat` — Get unread chats
- `qcc_search_company` — Company search
- `qcc_company_detail` — Company detail

## Architecture

### Overview

This is a recruitment assistant agent that provides an MCP (Model Context Protocol) server interface to interact with two Chinese platforms:

1. **Boss 直聘** (zhipin.com) — Job search, position details, HR chat
2. **企查查** (qcc.com) — Company information lookup

### Key Components

```
src/
├── main.rs              # Entry point, starts MCP server (stdio or streamable_http)
├── mcp_server.rs        # MCP tool definitions using rmcp framework
├── browser.rs           # ChromiumPage singleton management
├── config.rs            # YAML config loading from config.yaml
├── boss/                # Boss 直聘 platform integration
│   ├── handler/         # Login, search_position, position_detail, chat, send_message
│   └── model.rs        # LoginType enum
├── qcc/                 # 企查查 platform integration
│   ├── handler/         # Login, login_check, search_company, company_detail
│   └── model.rs
├── utils/               # Shared utilities
│   ├── job_query.rs    # JobSearchParams, URL building from name → code resolution
│   ├── site.rs         # City code lookups (101010100 = 北京)
│   ├── industry.rs     # Industry code lookups (100020 = 互联网)
│   ├── position.rs     # Position/职能 code lookups
│   ├── conditions.rs   # Salary, experience, education, scale, stage codes
│   └── salary.rs       # Salary parsing utilities
└── bin/                 # Standalone handler test binaries
```

### MCP Tools

The server exposes these tools (defined in `mcp_server.rs`):

**Boss 直聘:**
- `search_positions` — Search jobs via URL
- `get_job_detail` — Get position details from URL
- `boss_login` / `check_boss_login` — Login flow
- `start_new_chat` — Initiate chat from position URL
- `get_unread_chats` / `get_chat_messages` — Chat message handling

**企查查:**
- `qcc_login` / `check_qcc_login` — Login flow
- `search_qcc_company` — Search by keyword
- `get_qcc_company_detail` — Get company details from URL

**Lookup/Filter tools:**
- `search_cities` — City name → code
- `search_industries` — Industry name → code
- `search_position_codes` — Position name → code
- `list_filter_group` — Get all options for salary/experience/education/scale/stage

### Browser Management

The `browser.rs` singleton pattern manages a single `ChromiumPage` instance:
- Initialized lazily on first use via `browser::init()`
- Operations via `browser::with_browser(|page| ...)` closure
- Config-driven: `user_data_dir` for session persistence, `browser_exe_path` for Chrome path
- Thread-safe via `OnceLock` + `Mutex`

### Platform Flow Patterns

**Boss 直聘 job search:**
```
check_boss_login → search_positions → get_job_detail → start_new_chat → get_chat_messages
```

**企查查 company lookup:**
```
check_qcc_login → qcc_login → search_qcc_company → get_qcc_company_detail
```

### Config

`config.yaml` controls browser and MCP settings. If missing, a default config is auto-created:
- `user_data_dir` — Browser session directory (persists login state)
- `browser_exe_path` — Optional explicit Chrome path
- `qr_output_path` — Where to save QR code images
- `mcp.transport` — `streamable_http` (default, HTTP on port 8080) or `stdio`

### Dependencies

- `rust_drission` — Browser automation (ChromiumPage)
- `rmcp` — MCP server framework with macros (`projects/rust-sdk/crates/rmcp`)
- `axum` — HTTP server for streamable_http transport
- `tokio` — Async runtime
- `serde` / `serde_yaml` — Config and JSON serialization

### Tests

Unit tests use a `FakeBrowser` mock to test the `BrowserState` pattern without a real browser. Integration tests for handlers require actual browser/session state.
