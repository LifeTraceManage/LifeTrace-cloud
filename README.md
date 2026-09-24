# LifeTrace Cloud

LifeTrace 的轻量自托管后端与 Web 单仓库。Cloud 运行时只使用 Rust、Axum、SQLx 与 SQLite；Web 由独立 Nginx 容器提供静态资源并同源反向代理 Cloud API。BeeCount 兼容入口、邮件后台同步和 Execution 后台任务仍由 Cloud 进程提供。

目标不是做通用云平台，而是让个人服务器上的 LifeTrace 具备可靠同步、认证和少量云端能力，同时尽可能降低部署和维护成本。

## 当前架构

```text
Browser
   │
   ▼
LifeTrace Web :80
├── React/Vite static assets
└── Nginx /api + /health proxy
   │
   ▼
LifeTrace Cloud :8787
├── Axum API
├── Mail jobs
├── Exec jobs
├── SQLite
└── BeeCount :8869
   │
   ▼
/data/lifetrace.db
```

自托管环境使用两个职责单一的容器：`web` 和 `cloud`，外加一个 Cloud SQLite 数据卷。没有 PostgreSQL、独立 migration container、mail worker container 或 execution worker container。

SQLite 使用 WAL 模式，数据库 migration 在 Cloud 启动时自动执行。

## 目录

- `src/routes/`：HTTP API
- `src/auth/`：认证、Session、Token 与密码逻辑
- `src/beecount/`：BeeCount 兼容和财务同步
- `src/mail/`：邮件协议、解析和邮件服务
- `src/workers/`：随 Cloud 进程启动的后台任务
- `src/repository/sqlite/`：Sync v1 SQLite 持久化
- `src/sync/`：游标、分页令牌和 payload hash
- `migrations/0001_sqlite.sql`：SQLite 基线 schema
- `migrations/0002_mail_workspace.sql`：Mail Workspace 增量 schema（Identity / Draft attachment）
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

## 双镜像部署

Web 与 Cloud 源码仍然位于同一个仓库，但 GitHub Actions 独立构建两个镜像：

```text
ghcr.io/lifetracemanage/lifetrace-web:main
ghcr.io/lifetracemanage/lifetrace-cloud:main
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

服务器只需要 Docker / Docker Compose，不需要 Node、Rust 或源码构建工具。

首次部署：

```bash
cp deploy/cloud/.env.production.example deploy/cloud/.env.production
# 编辑密钥和 PUBLIC_WEB_BASE_URL / CORS_ALLOWED_ORIGINS

docker login ghcr.io   # 仅当 package 需要认证时
bash deploy/cloud/deploy-production.sh
bash deploy/cloud/verify-production.sh
```

部署脚本只执行两个镜像的 pull 和 Compose 更新：

```bash
docker compose --env-file .env.production \
  -f deploy/cloud/docker-compose.production.yml pull cloud web

docker compose --env-file .env.production \
  -f deploy/cloud/docker-compose.production.yml up -d --remove-orphans --wait
```

可以分别固定 Web/Cloud 到不同 SHA，独立升级和回滚：

```text
LIFETRACE_WEB_IMAGE=ghcr.io/lifetracemanage/lifetrace-web:sha-<commit>
LIFETRACE_CLOUD_IMAGE=ghcr.io/lifetracemanage/lifetrace-cloud:sha-<commit>
```

持久化只属于 Cloud：

```text
lifetrace_data -> /data/lifetrace.db
```

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

1. SQLite 是唯一数据库后端，不维护 PostgreSQL / SQLite 双栈；生产服务器只拉取 Actions 构建好的 Web/Cloud 镜像。
2. 测试同样使用 SQLite，不维护第二套内存数据库实现。
3. 后台任务运行在 Cloud Tokio runtime 内，不拆独立 worker 服务。
4. 管理命令与服务端共用 `lifetrace-cloud` 一个二进制，不维护独立 admin/migration/worker 入口。
5. Web 使用独立 Nginx 容器提供 SPA，并同源反代 Cloud API；Cloud 不再承担前端静态资源。
6. 数据库 schema 使用精简的 SQLite migration 链：一个当前基线 + 必要的向前兼容增量；不保留 PostgreSQL 历史 migration 链。
7. Sync v1 wire contract 保持兼容，数据库实现细节不暴露给客户端。
