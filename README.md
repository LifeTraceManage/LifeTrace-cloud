# LifeTrace Cloud

LifeTrace 的轻量自托管后端与 Web 单仓库。Cloud 运行时只使用 Rust、Axum、SQLx 与 SQLite；Web 由独立 Nginx 容器提供静态资源并同源反向代理 Cloud API。BeeCount 兼容入口、邮件后台同步和 Execution 后台任务仍由 Cloud 进程提供。

目标不是做通用云平台，而是让个人服务器上的 LifeTrace 具备可靠同步、认证和少量云端能力，同时尽可能降低部署和维护成本。

## 当前架构

```text
Browser
   │ HTTPS / HTTP
   ▼
Caddy :443 / :80
   │
   ▼
LifeTrace Web (Nginx) :80
├── React/Vite static assets
└── Nginx /api + /health proxy
   │
   ▼
LifeTrace Cloud :8787
├── Axum API
├── Agent runtime (Rig)
├── Mail jobs
├── Exec jobs
├── SQLite
└── BeeCount :8869
   │
   ▼
/data/lifetrace.db
```

自托管环境使用三个职责单一的容器：`caddy` 负责公网 HTTP/HTTPS，`web` 提供静态资源和同源 API 代理，`cloud` 提供 Rust API、后台任务和 SQLite。没有 PostgreSQL、独立 migration container、mail worker container 或 execution worker container。

SQLite 使用 WAL 模式，数据库 migration 在 Cloud 启动时自动执行。

## 目录

- `src/routes/`：HTTP API
- `src/auth/`：认证、Session、Token 与密码逻辑
- `src/agent/`：Rig Agent runtime、会话持久化、数据工具与审批执行层
- `src/beecount/`：BeeCount 兼容和财务同步
- `src/mail/`：邮件协议、解析和邮件服务
- `src/workers/`：随 Cloud 进程启动的后台任务
- `src/repository/sqlite/`：Sync v1 SQLite 持久化
- `src/sync/`：游标、分页令牌和 payload hash
- `migrations/0001_sqlite.sql`：SQLite 基线 schema
- `migrations/0002_mail_workspace.sql`：Mail Workspace 增量 schema（Identity / Draft attachment）
- `migrations/0003_agent_runtime.sql`：Agent session / run / message / tool call / approval schema
- `apps/web/`：LifeTrace Web 正式源码；与 Cloud 在同一仓库、同一镜像中构建
- `apps/web/Dockerfile` / `apps/web/nginx.conf`：Web 独立镜像与同源 API 代理
- `deploy/cloud/`：生产双服务 Compose、部署脚本与环境变量模板
- `crates/lifetrace-contracts/`：共享协议和领域契约
- `crates/lifetrace-sync-client/`：Rust Sync v1 客户端
- `contracts/`：生成的跨语言契约

## HTTP 入口

Cloud 主服务监听容器内部 `8787`；生产环境由 Web Nginx 容器监听宿主机 `80` 并反向代理 `/api/*`、`/health/*` 到 Cloud。

| 路径 | 能力 |
| --- | --- |
| `/health/*` | 健康检查 |
| `/api/v1/auth/*` | 登录、Session、Token |
| `/api/v1/sync/*` | Push / Pull / Snapshot |
| `/api/v1/files/*` | 文件元数据和对象存储签名 |
| `/api/v1/privacy/*` | 数据导出和账号删除 |
| `/api/v1/integrations/beecount/*` | LifeTrace Web 财务接口 |
| `/api/v1/mail/*` | 邮件 |
| `/api/v1/assistant` | Agent 对话（Web/Native 共用 runtime） |
| `/api/v1/assistant/sessions` | Agent 历史会话 |
| `/api/v1/assistant/sessions/{id}` | DELETE 删除 Agent 会话及其消息、Run、工具调用与审批记录 |
| `/api/v1/assistant/sessions/{id}/messages` | Agent 会话消息 |
| `/api/v1/assistant/sessions/{id}/approvals` | Agent 会话写操作审批 |
| `/api/v1/assistant/approvals/{id}/decision` | 批准/拒绝 Agent 写操作 |
| `/api/v1/photo-*/*` | 照片相关能力 |
| 其他路径 | Cloud 不负责 SPA；由 `web` 容器提供 |

BeeCount 兼容监听 `8869`。该监听器只负责把 BeeCount 原始 `/api/v1/*` 和 `/ws` 请求重写到内部兼容路由，不需要 Caddy。

## 本地运行

默认数据库文件：

```text
./data/lifetrace.db
```

直接运行：

```bash
cargo run --bin lifetrace-cloud
```

管理命令也使用同一个二进制：

```bash
cargo run --bin lifetrace-cloud -- bootstrap-user --email you@example.com
cargo run --bin lifetrace-cloud -- create-invite --email someone@example.com
```

自定义数据库位置：

```bash
export LIFETRACE_DATABASE_PATH=/tmp/lifetrace.db
cargo run --bin lifetrace-cloud
```

首次启动会创建 SQLite 文件并执行全部尚未应用的版本化 migration；已执行的 migration 不会被重写。

## 测试

测试不需要 PostgreSQL 或其他外部数据库服务：

```bash
export TEST_DATABASE_PATH=/tmp/lifetrace-test.db
cargo fmt --check
cargo test --locked -- --test-threads=1
cargo clippy --locked --all-targets -- -D warnings
cargo test --manifest-path crates/lifetrace-contracts/Cargo.toml
cargo test --manifest-path crates/lifetrace-sync-client/Cargo.toml
cargo run --manifest-path tools/contract-exporter/Cargo.toml
```

## 本地构建部署

### 项目根目录一键部署

服务器已经 clone 本仓库并配置过生产环境后，进入项目根目录直接执行：

```bash
./deploy.sh
```

它会依次完成：

1. 检查 Git、Docker Engine 与 Docker Compose v2；
2. 检查 tracked 文件是否存在未提交修改，避免部署时误覆盖；
3. 从 `origin/main` 拉取最新代码，并重新载入最新版部署脚本；
4. 运行 `deploy/cloud/deploy-production.sh init-env`，只补充新增环境变量，不覆盖现有生产密钥；
5. 本地构建并启动 Cloud、Web 与 Caddy；
6. 执行生产健康检查并验证 SQLite 在 Cloud 重启后仍保持持久化；
7. 输出当前 commit、HTTPS 地址和 IP fallback 地址。

也可以只部署单个应用镜像：

```bash
./deploy.sh web
./deploy.sh cloud
```

如果服务器上 intentionally 保留了 tracked 本地修改，可明确跳过源码更新，仅部署当前 checkout：

```bash
LIFETRACE_SKIP_UPDATE=true ./deploy.sh
```

如需跳过部署后的验证：

```bash
LIFETRACE_SKIP_VERIFY=true ./deploy.sh
```

Web 与 Cloud 源码位于同一个仓库，生产环境默认直接从当前源码构建两个本地 Docker 镜像：

```text
lifetrace-web-app:local
lifetrace-cloud:local
```

生产拓扑：

```text
Internet
   │
   ▼
web container :80
├── React/Vite
├── /photo-challenge-upload
└── /api/* + /health/* ─────► cloud container :8787
                                  ├── Axum API
                                  ├── Mail / Execution jobs
                                  └── SQLite /data/lifetrace.db

BeeCount clients ─────────────► host :8869 → cloud :8869
```

Web Nginx 将附件上传上限设为 256 MiB，并把 API 保持为同源反向代理，因此浏览器认证 Cookie 不需要跨域配置。

服务器需要 Docker Engine、Docker Compose v2 和 Git。构建阶段还需要能够获取 Docker 基础镜像以及 npm / crates.io 依赖；运行阶段不依赖 GHCR。

### 一键安装 / 更新

安装器会在服务器上 clone 最新源码，然后使用 Docker Compose 本地构建 Web 与 Cloud：

```bash
curl -fsSL \
  https://raw.githubusercontent.com/LifeTraceManage/LifeTrace-cloud/main/deploy/cloud/install.sh \
  | bash -s -- --base-url http://YOUR_SERVER_IP
```

如果已经配置 HTTPS：

```bash
curl -fsSL \
  https://raw.githubusercontent.com/LifeTraceManage/LifeTrace-cloud/main/deploy/cloud/install.sh \
  | bash -s -- --base-url https://lifetrace.example.com
```

默认目录：

```text
~/lifetrace/source
```

安装器会：

- 检查 Git、Docker Engine 和 Docker Compose v2；
- clone 或更新 `LifeTrace-cloud` 源码；
- 首次安装自动生成 Cursor、Page Token、Password Pepper、Token Pepper 四个随机密钥；
- 已存在旧版 `~/lifetrace/.env.production` 时自动迁移并保留原密钥；
- 将生产镜像固定为 `lifetrace-web-app:local` 和 `lifetrace-cloud:local`；
- 在服务器本地执行 Docker build；
- 启动两个容器并验证 SQLite 持久化、Web 反向代理和健康检查。

再次运行同一条安装命令会拉取最新源码并重新构建发生变化的 Docker 层，不会删除 SQLite volume，也不会重新生成已有密钥。

### 日常发布

进入部署目录：

```bash
cd ~/lifetrace/source/deploy/cloud
```

代码更新后先同步生产环境变量模板。该命令可重复执行：已有 `.env.production` 时只追加模板中新出现的变量，不覆盖现有密钥或自定义值；首次部署时则从 `.env.production.example` 创建配置文件。

```bash
./deploy-production.sh init-env
```

全部重新构建并发布：

```bash
./deploy-production.sh all
```

只修改 Web 时：

```bash
./deploy-production.sh web
```

只修改 Rust Cloud 时：

```bash
./deploy-production.sh cloud
```

部署脚本使用：

```bash
docker compose build <service>
docker compose up -d --remove-orphans --wait --pull never
```

因此不会访问 GHCR。Docker layer cache 会复用未发生变化的依赖层。

如果服务器上的源码不是最新版本，先更新：

```bash
cd ~/lifetrace/source
git fetch origin main
git checkout main
git pull --ff-only

cd deploy/cloud
./deploy-production.sh all
```

也可以直接重新运行一键安装命令，让安装器负责更新源码。

### 日志与排障

Cloud 使用 Rust `tracing` 统一记录结构化运行日志。生产环境默认：

```text
RUST_LOG=info
```

实时查看后端：

```bash
cd ~/lifetrace/source/deploy/cloud
docker compose --env-file .env.production \
  -f docker-compose.production.yml \
  logs -f --tail=200 cloud
```

默认 `INFO` 会显示 HTTP 请求、邮件实际变更、邮件操作、后台任务异常等有价值事件；健康检查、邮箱轮询无变化等噪声只记录到 `DEBUG`。

临时排查 Mail 时可以在 `.env.production` 中设置：

```text
RUST_LOG=info,lifetrace::mail=debug
```

排查 HTTP：

```text
RUST_LOG=info,lifetrace::http=debug
```

修改后重新部署 Cloud 即可。生产 Compose 对 Cloud 的 Docker 日志启用了轮转：单文件 20 MiB，最多保留 5 个文件，避免长期运行耗尽磁盘。

邮件操作日志只记录内部 UUID、动作、状态和耗时，不应记录密码、Cookie、Authorization、邮箱授权码或邮件正文。

### 手动首次部署

```bash
git clone https://github.com/LifeTraceManage/LifeTrace-cloud.git
cd LifeTrace-cloud/deploy/cloud

./deploy-production.sh init-env
# 首次创建后检查随机密钥、域名/IP、PUBLIC_WEB_BASE_URL 和 CORS_ALLOWED_ORIGINS

./deploy-production.sh all
./verify-production.sh
```

生产 Compose 默认配置：

```text
LIFETRACE_WEB_IMAGE=lifetrace-web-app:local
LIFETRACE_CLOUD_IMAGE=lifetrace-cloud:local
```

SQLite 持久化只属于 Cloud：

```text
lifetrace_data -> /data/lifetrace.db
```

GitHub Actions 仍可继续构建 GHCR 镜像作为可选发布产物，但默认服务器部署流程不再依赖这些远程镜像。

## Agent

Cloud 内置基于 Rig 的 Agent runtime。会话、消息、Run、工具调用与写操作审批都保存在同一 SQLite 中；模型只负责推理、读取工具调用和生成结构化写操作提案，不拥有绕过认证或直接写数据库的能力。

读取工具包括：

- LifeTrace 实体概览；
- 按 entity type / 关键词搜索任务、日程、笔记、习惯、复盘、训练、财务等 Sync 数据；
- 搜索近 30 天邮件元数据和摘要。

当前审批写操作白名单包括：

- 创建任务；
- 修改任务标题、状态、优先级、截止时间或 Planner 执行时间段；
- 创建日程；
- 创建/修改 Project；
- 创建/修改习惯（名称、目标、执行日、开始日期、说明与归档状态）；
- 创建/修改 Memo；
- 创建/修改 Waiting Item；
- 为 task / calendar_event / waiting_item / memo 创建或修改 Reminder。

写操作采用 `Agent propose -> user approval -> deterministic backend execution`。propose 工具只写入 `agent_approvals` 和审计记录，不修改业务数据；用户在 Web 中显式批准后，Cloud 再通过现有 `SyncRepository.push` 执行，因此仍使用 Sync v1 的版本、冲突、change log 和客户端同步机制。批准请求默认 15 分钟过期，重复批准按 approval/action 的稳定标识处理为幂等操作。

如果用户在批准前修改某个提案，Agent 会先读取当前会话的 pending approvals，再用新提案的 `supersedesApprovalId` 显式替代旧 approval。替代在同一 SQLite 事务中完成：新 approval 创建成功后，旧 approval 立即变为 `cancelled`，记录 `cancellationReason=superseded` 和 `supersededByApprovalId`，旧 tool call 同步变为 denied。旧 approval 之后不能再批准；不同 `actionName` 的审批不能互相替代，避免误撤销无关操作。

任务规划遵循 `dueAt != scheduledStartAt/scheduledEndAt`：截止时间只表示 deadline，Planner 时间段表示实际执行计划。对于用户给出明确 deadline 的任务，Agent 默认会先读取目标日期附近的任务和日程，主动选择无冲突的执行时段并把计划时间放入同一审批提案；用户明确要求“只收集、稍后再排”时才保留为待安排。已有任务也可直接通过修改 `scheduledStartAt/scheduledEndAt` 进入 Planner，不再需要额外创建 Calendar Event。

审批同时要求同一 `user_id`、同一 `app_id + scopes` 分区，并在执行时按 action 重新校验当前 Session 的写权限：任务、日程、Project、Memo、Waiting Item、Reminder 要求 `sync:write + execution:write`，习惯要求 `sync:write + habits:write`。Reminder 执行时还会重新确认 subject 实体仍存在，避免产生悬空提醒。前端只提交 approval id 和 approve/reject，不接受模型生成的任意 API/SQL。删除数据、发送/删除邮件及其他高风险写操作目前仍未开放。

Web Assistant 将会话列表和聊天正文做独立滚动；会话支持显式删除。审批区只在主视图展示待处理项，已批准/拒绝/过期项折叠为最近审批历史。会话删除会利用 SQLite 外键级联清理该会话的 message、run、tool call 和 approval，避免长期留下孤儿记录。

每次工具调用都同时绑定当前 `user_id` 和当前认证 Session 的 scopes。旧 Web 客户端仍可发送 `context` 字段，但服务端不再把客户端拼装的 context 当作可信数据源。

模型配置统一使用以下环境变量：

```text
MODEL_PROVIDER=deepseek
MODEL_API_KEY=...
MODEL_BASE_URL=https://api.deepseek.com
MODEL_NAME=deepseek-chat
```

`MODEL_PROVIDER` 当前支持：

- `deepseek`：使用 Rig 原生 DeepSeek provider；
- `qwen`：通过 DashScope OpenAI-compatible 接口运行；
- `openai`：使用 OpenAI-compatible client，默认 base URL 为 `https://api.openai.com/v1`，需要显式配置 `MODEL_NAME`；
- `openai-compatible`：用于其他兼容 Chat Completions 的服务或自建网关，需要显式配置 `MODEL_BASE_URL` 和 `MODEL_NAME`。

Qwen 示例：

```text
MODEL_PROVIDER=qwen
MODEL_API_KEY=...
MODEL_BASE_URL=https://dashscope.aliyuncs.com/compatible-mode/v1
MODEL_NAME=qwen-plus
```

自定义兼容服务示例：

```text
MODEL_PROVIDER=openai-compatible
MODEL_API_KEY=...
MODEL_BASE_URL=https://your-provider.example/v1
MODEL_NAME=your-model
```

旧的 `DEEPSEEK_API_KEY`、`DEEPSEEK_BASE_URL`、`DEEPSEEK_MODEL` 在 provider 为 `deepseek` 时仍兼容，便于已有服务器平滑升级。新部署优先使用 `MODEL_*`。

未配置 `MODEL_API_KEY`（且没有兼容的旧 DeepSeek Key）时接口仍可用，会保存会话并返回 local fallback；配置后使用 Rig 的多轮 Agent loop、读取工具与审批提案工具。

Agent 的 INFO 日志只记录 run/session/tool call/approval ID、provider、model、状态与错误，不记录 prompt、邮件正文或工具结果正文。排障时可在 `.env.production` 中提高 Agent 模块日志级别。

## 对象存储

较大的长期文件仍可使用 S3 兼容对象存储。SQLite 保存 `file_objects` 元数据，Cloud 签发短期上传/下载 URL，不代理长期对象字节。

相关可选配置：

```text
FILE_OBJECT_STORAGE_ENDPOINT
FILE_OBJECT_STORAGE_BUCKET
FILE_OBJECT_STORAGE_REGION
FILE_OBJECT_STORAGE_ACCESS_KEY_ID
FILE_OBJECT_STORAGE_SECRET_ACCESS_KEY
FILE_OBJECT_STORAGE_PRESIGN_TTL_SECONDS
FILE_MAX_UPLOAD_BYTES
```

没有配置对象存储时，不影响核心同步、认证、Web、邮件和 BeeCount 功能。

## Notes / Mail 兼容性

SQLite 单容器重构保持 Web 已上线的 Notes / Mail 契约，不以“简化部署”为理由删减产品能力。

Notes 保留：

- `note.folder.parentFolderId` 层级目录契约；
- Note / Tag / Relation / Revision 等 Sync v1 entity；
- Web 的 Wiki Link、Backlinks、Properties、Revision History 与 Calendar/Mail 联动无需后端分叉。

Mail 保留：

- Account / Identity / Draft；
- Unified Inbox、Mailbox Role、Starred 与正文/发件人/收件人搜索；
- Read / Star / MOVE；
- Identity Display Name / Reply-To / Signature；
- Draft attachment（SQLite BLOB，单封总量 18 MiB 上限）；
- SMTP multipart send；
- SMTP 成功后按 Message-ID 检查 Sent，Provider 未自动保存时才通过 IMAP APPEND 补副本，避免重复；
- Mail privacy export 中的 Identity / Draft metadata。

`MAIL_CREDENTIAL_KEY` 未配置时只关闭邮件聚合后台任务，不影响 Cloud 的其他功能。

## 设计原则

当前仓库按单实例个人服务器优化。Web 正式源码位于 `apps/web`，但 Web 与 Cloud 以两个独立镜像发布：

1. SQLite 是唯一数据库后端，不维护 PostgreSQL / SQLite 双栈；生产服务器默认从仓库源码本地构建 Web/Cloud 镜像，不依赖 GHCR。
2. 测试同样使用 SQLite，不维护第二套内存数据库实现。
3. 后台任务运行在 Cloud Tokio runtime 内，不拆独立 worker 服务。
4. 管理命令与服务端共用 `lifetrace-cloud` 一个二进制，不维护独立 admin/migration/worker 入口。
5. Web 使用独立 Nginx 容器提供 SPA，并同源反代 Cloud API；Cloud 不再承担前端静态资源。
6. 数据库 schema 使用精简的 SQLite migration 链：一个当前基线 + 必要的向前兼容增量；不保留 PostgreSQL 历史 migration 链。
7. Sync v1 wire contract 保持兼容，数据库实现细节不暴露给客户端。
