# Txffp Agent (票根自动开票工具)

小型、高可靠、低资源占用、可长期维护的票根网 ([txffp.com](https://www.txffp.com/)) 自动化开票 Agent。

## 核心设计理念

- **自动优先**：配置合法凭据后优先通过正常登录流程与会话复用完成开票。
- **人工兜底**：遇到短信验证码、滑块验证或未知风控页面时，自动建立 Human Action 任务并通过 `/human/{token}` 页面请求用户介入接管。
- **研究驱动**：Scrapling 作为逆向与脱敏研究工具，生产环境优先走精简高效的 HTTP 接口，必要时通过 Steel Browser 驱动。
- **结果验证**：无论用户是否声称完成验证，系统均调用业务只读接口进行二次鉴实验证。
- **失败可恢复**：任务中断与状态持久化保存于 SQLite，登录恢复后自动重新拉取最新记录并继续。
- **极简依赖**：Rust 后端 + SQLite 单文件，无 PostgreSQL/Redis 等重型依赖。

## 架构

```text
AI (MCP / REST)
       │
       ▼
  txffp-server (Rust + Axum + SQLite)
       │
  ┌────┴──────────────┐
  ▼                   ▼
票根 HTTP API     Steel Browser (192.168.8.11:13000)
                      │
                      ▼
                  txffp.com
```

## 快速开始

### 环境变量配置

复制 `.env.example` 为 `.env`：

```bash
cp .env.example .env
```

配置票根账号凭据与外部服务：

```env
TXFFP_USERNAME=your_username
TXFFP_PASSWORD=your_password
STEEL_BASE_URL=http://192.168.8.11:13000
DATABASE_URL=sqlite:///data/txffp.db
```

### 本地编译运行

```bash
# 检查
cargo check

# 测试
cargo test

# 运行
cargo run -p txffp-server
```

### Docker 运行

```bash
# 启动后端核心服务
docker compose -f docker/docker-compose.yml up -d

# 需要启用可选管理前端
docker compose -f docker/docker-compose.yml --profile ui up -d
```

## MCP 工具

| 工具名 | 说明 |
|---|---|
| `txffp_auth_status` | 查询当前登录状态与凭据配置 |
| `txffp_auth_ensure` | 确保会话有效，必要时自动登录或生成人工处理通道 |
| `txffp_list_cards` | 获取用户 ETC 卡列表 |
| `txffp_invoice_preview` | 预览指定日期范围待开票记录（只读，不生成发票） |
| `txffp_invoice_create` | 提交真实开票（幂等防重复保护） |
| `txffp_invoice_list` | 查询已开具发票与下载链接 |

## 授权与安全

- 严禁在日志、Tracing 或接口中暴露明文密码。
- 不采用暴力破解、绕过验证码等黑客手段。
