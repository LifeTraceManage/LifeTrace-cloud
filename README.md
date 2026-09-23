# LifeTrace Cloud

LifeTrace 的轻量自托管云端服务。当前运行时只使用 Rust、Axum、SQLx 与 SQLite；Web 静态资源、BeeCount 兼容入口、邮件后台同步和 Execution 后台任务都由同一个 Cloud 进程提供。

目标不是做通用云平台，而是让个人服务器上的 LifeTrace 具备可靠同步、认证和少量云端能力，同时尽可能降低部署和维护成本。

## 当前架构

```text
Browser / Mobile / Desktop / BeeCount
               │
        ┌──────┴──────┐
        │ LifeTrace   │
        │ Cloud       │
        │             │
        │ Axum API    │ :8787
        │ BeeCount    │ :8869
        │ Web static  │
        │ Mail jobs   │
        │ Exec jobs   │
        │ SQLite      │
        └──────┬──────┘
               │
         /data/lifetrace.db
```

自托管环境只有一个常驻应用进程和一个持久化数据目录。没有 PostgreSQL、Caddy、migration container、mail worker container 或 execution worker container。

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
- `deploy/cloud/`：唯一生产 Compose、部署脚本与环境变量模板
- `crates/lifetrace-contracts/`：共享协议和领域契约
- `crates/lifetrace-sync-client/`：Rust Sync v1 客户端
- `contracts/`：生成的跨语言契约

## HTTP 入口

主服务监听 `8787`，生产 Compose 映射为宿主机 `80`。

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
| 其他路径 | Web SPA 静态资源 |

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

## 单容器部署

生产部署只有一个完整镜像和一个 service：

```text
ghcr.io/lifetracemanage/lifetrace-cloud:main
├── /app/lifetrace-cloud
├── /app/web
│   ├── index.html
│   └── assets/
├── /app/photo-challenge
└── /data
    ├── lifetrace.db
    └── photo-staging/
```

Web 正式源码现在位于本仓库 `apps/web/`。GitHub Actions 在同一个 Docker build 中先执行 Web 的 `npm ci / typecheck / test / build`，再编译 Rust Cloud，并把 Web `dist` 与 Photo Challenge 静态资源复制进最终镜像。

服务器不再需要 Node、Rust、LifeTrace-web 独立 checkout 或本地构建环境，只需要 Docker / Docker Compose。

首次部署：

```bash
cp deploy/cloud/.env.production.example deploy/cloud/.env.production
# 编辑密钥和服务器地址

# 如果 GHCR package 是 private，先登录：
echo <GITHUB_TOKEN> | docker login ghcr.io -u <GITHUB_USER> --password-stdin

bash deploy/cloud/deploy-production.sh
bash deploy/cloud/verify-production.sh
```

部署脚本等价于：

```bash
cd deploy/cloud
docker compose --env-file .env.production -f docker-compose.production.yml pull lifetrace
docker compose --env-file .env.production -f docker-compose.production.yml up -d --remove-orphans --wait
```

默认镜像：

```text
ghcr.io/lifetracemanage/lifetrace-cloud:main
```

也可以在 `.env.production` 固定到某个 Actions 生成的 SHA 镜像：

```text
LIFETRACE_CLOUD_IMAGE=ghcr.io/lifetracemanage/lifetrace-cloud:sha-<commit>
```

固定 SHA 更适合稳定生产部署和回滚；`:main` 适合持续跟踪主分支。

服务端口：

```text
80   -> container:8787   LifeTrace Web + API
8869 -> container:8869   BeeCount compatibility
```

持久化只需要备份 Docker volume 中的 `/data`。数据库主体是 `lifetrace.db`；使用 WAL 时，在线备份应通过 SQLite backup/checkpoint 语义完成，而不是在高写入期间只复制主数据库文件。

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

当前 Cloud 明确按单实例个人服务器优化。Web 正式源码已合并到 `apps/web`，Actions 直接发布包含 Web + Cloud 的完整镜像：

1. SQLite 是唯一数据库后端，不维护 PostgreSQL / SQLite 双栈；生产服务器默认只拉取 GitHub Actions 构建好的完整镜像。
2. 测试同样使用 SQLite，不维护第二套内存数据库实现。
3. 后台任务运行在 Cloud Tokio runtime 内，不拆独立 worker 服务。
4. 管理命令与服务端共用 `lifetrace-cloud` 一个二进制，不维护独立 admin/migration/worker 入口。
5. Web 静态资源由 Axum 提供，不增加 Caddy/Nginx 依赖.
6. 数据库 schema 使用精简的 SQLite migration 链：一个当前基线 + 必要的向前兼容增量；不保留 PostgreSQL 历史 migration 链。
7. Sync v1 wire contract 保持兼容，数据库实现细节不暴露给客户端。
