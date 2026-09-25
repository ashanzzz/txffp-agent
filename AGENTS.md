# AGENTS.md - Txffp Agent 开发指南与架构规范

## 1. 项目目标

开发一个小型、高可靠、低资源占用、可长期维护的票根自动开票工具 (Txffp Agent)。
- 目标网站: [https://www.txffp.com/](https://www.txffp.com/)
- 仅操作用户本人有权访问的票根账户、ETC 卡、通行记录与发票。
- 主要交互方式: AI -> MCP / REST API -> txffp-server -> 票根网。
- 遵循准则: **自动优先、人工兜底、研究驱动、结果验证、失败可恢复、状态可持续**。

## 2. 目录结构

```text
.
├── Cargo.toml            # Rust workspace 配置
├── server/               # Rust 后端 (txffp-server)
│   ├── Cargo.toml
│   └── src/
│       ├── main.rs       # 启动入口与服务编排
│       ├── config.rs     # 配置管理 (环境变量与 Secret 文件)
│       ├── credential.rs # CredentialProvider (环境/文件/设置)
│       ├── db.rs         # SQLite 存储层 (operations, downloads, human_actions, kv)
│       ├── auth/         # 认证状态机与 AuthManager
│       ├── human_action/ # Human Action 管理器与内置 /human/{token} 页面
│       ├── browser/      # BrowserDriver 与 Steel Browser 适配器
│       ├── txffp/        # 票根业务逻辑 (ETC卡、通行记录、发票抬头、开票、下载)
│       ├── api/          # Axum REST API 路由与统一响应封装
│       └── mcp/          # MCP (Streamable HTTP) Server 实现
├── web/                  # React 前端 (txffp-web, 可选独立组件)
├── docs/                 # 架构设计、研究笔记与协议文档
├── docker/               # Dockerfile 与 Docker Compose 配置
├── data/                 # 本地运行时数据 (SQLite, secrets, downloads, research)
├── .env.example          # 环境变量示例
└── AGENTS.md             # 本规范文档
```

## 3. 总体架构与数据流

```text
                 ┌─────────────────┐
                 │ AI / MCP Client │
                 └────────┬────────┘
                          │
                     MCP / REST
                          │
                          ▼
            ┌──────────────────────────┐
            │       txffp-server       │
            │           Rust           │
            │                          │
            │ REST API                 │
            │ MCP Server               │
            │ TxffpService             │
            │ Auth Manager             │
            │ HTTP Adapter             │
            │ Browser Adapter          │
            │ Human Action Manager     │
            │ SQLite (/data/txffp.db)  │
            │ Download Manager         │
            └────────────┬─────────────┘
                         │
              ┌──────────┴───────────┐
              │                      │
              ▼                      ▼
       Txffp HTTP API          Steel Browser
                                      │
                                      ▼
                                 txffp.com
```

## 4. 依赖与外部服务

- **后端**: Rust stable, Tokio, Axum, Serde, Reqwest, SQLx (SQLite), rust_decimal, Tracing.
- **数据库**: 单文件 SQLite (`/data/txffp.db`)。严禁引入 PostgreSQL、Redis、RabbitMQ 等臃肿中间件。
- **Steel Browser**: 远程无头浏览器环境（Unraid 宿主默认地址 `http://192.168.8.11:13000`）。用于登录驱动与复杂安全验证接管。
- **Scrapling**: 仅作为页面逆向与脱敏抓取的研究辅助工具，不参与生产运行与核心 Session 管理。
- **React Frontend**: 可选管理端，关闭不影响后端 REST、MCP、自动化开票与内置 Human Action。

## 5. 认证状态机 (Authentication State Machine)

认证状态严格划分，严禁使用单一布尔值 `logged_in`:
- `UNKNOWN`: 未知初始态
- `CHECKING`: 正在验证会话
- `CREDENTIALS_REQUIRED`: 缺少账号密码凭据
- `LOGGED_OUT`: 已登出
- `AUTO_LOGIN`: 正在执行自动化登录
- `LOGIN_REQUIRED`: 需要登录
- `LOGIN_IN_PROGRESS`: 登录流程中
- `VERIFICATION_REQUIRED`: 需要通用验证
- `MFA_REQUIRED`: 需要多因素验证
- `CAPTCHA_REQUIRED`: 需要图形/滑块验证码
- `HUMAN_ACTION_REQUIRED`: 需要用户介入处理
- `AUTH_UNVERIFIED`: 用户声称完成但机器校验尚未通过
- `LOGGED_IN`: 会话有效且已通过账户数据只读接口验证
- `SESSION_EXPIRED`: 会话已过期
- `RATE_LIMITED`: 触发风控频率限制 (429/操作频繁)
- `LOCKED`: 账号锁定
- `ERROR`: 异常错误

## 6. 凭据管理与安全边界 (Credential Provider)

- 支持来源：Docker Secret -> `/data/secrets.toml` -> 环境变量 (`TXFFP_USERNAME`, `TXFFP_PASSWORD`) -> 前端设置更新。
- 严禁在 GET API、MCP 工具返回值、日志、Tracing 或 Git 中输出明文密码。
- 前端只展示凭据配置状态 (`username_configured: true`, `password_configured: true`)。
- 登录安全边界：支持自动填表与正常登录提交，严禁暴力穷举、验证码自动破解或绕过行为；重试次数严格限制（默认最多 3 次）。

## 7. Human Action 协议与独立页面

- 当遇到短信验证码 (`SMS_CODE`)、滑块人机验证 (`CAPTCHA`)、设备确认 (`DEVICE_CONFIRM`) 等情况时，生成 Human Action 记录。
- 后端自带独立轻量页面：`/human/{token}`，展示当前原因、任务上下文、Steel 会话画面及“完成/取消”按钮。
- 用户点击完成后，后端必须再次调用远端业务只读接口验证登录状态，方可恢复执行原任务。

## 8. 开票安全规则 (Preview & Submit)

- **Preview 核心**: `POST /api/v1/invoices/preview` 必须为纯只读操作，绝不生成实际发票。
- **货币精度**: 人民币金额必须使用 `rust_decimal` 或整数“分”，严禁使用 `f32`/`f64`。
- **幂等保护**: 真实提交 `POST /api/v1/invoices` 必须携带 `idempotency_key`。
- **超时保护**: 真实提交发生网络超时或 502 时，状态置为 `REMOTE_RESULT_UNKNOWN`，严禁直接自动重新 POST，必须通过查询远端历史确认状态。
- **任务上下文恢复**: 登录失效暂停的任务保存在 SQLite `operations` 中，登录恢复后必须重新查询最新通行记录与计算金额，防止脏数据提交。

## 9. 研究模式 (Research Mode)

- 配置：`TXFFP_RESEARCH_MODE=true`，保存脱敏后的网络元数据与 DOM 结构至 `/data/research/`。
- 敏感信息必须自动脱敏（密码、验证码、Token、身份证、手机号、完整 ETC 卡号）。

## 10. Docker 与部署

- 后端基于 multi-stage 构建，最终镜像基于 `debian:bookworm-slim`，不内置 Chromium（由 Steel 提供）。
- 数据目录持久化挂载至宿主 `/mnt/user/appdata/txffp-agent/` -> 容器内 `/data`。
- GitHub Actions 在 PR 时执行 `cargo fmt --check`, `cargo clippy -- -D warnings`, `cargo test`。
- Release Tag 触发自动构建并推送到 GHCR。
