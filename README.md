# LifeTrace Cloud

LifeTrace 的独立云端后端，使用 Rust、Axum、SQLx 与 PostgreSQL。仓库负责跨端同步、认证、文件元数据与对象存储签名、BeeCount 财务兼容、邮件能力以及少量云端任务，不包含 Flutter / Desktop 客户端源码。

## 架构边界

```text
Client
  │
  ▼
Axum routes
  │
  ├─ auth / security
  ├─ sync v1
  ├─ files / privacy
  ├─ BeeCount compatibility
  ├─ mail
  └─ photo / assistant support
  │
  ▼
Application state
  │
  ├─ PostgreSQL repositories        production
  ├─ object storage signer
  ├─ auth service
  └─ in-memory sync repository      tests / protocol harness only
```

生产同步路径只使用 PostgreSQL。内存同步实现保留在 `src/repository/memory_store.rs`，用于无数据库的协议测试与快速验证，不属于生产持久化方案。

## 目录

- `src/`：服务端入口、配置、安全与通用基础设施
- `src/routes/`：HTTP 路由层
- `src/auth/`：认证、授权、Session、Token 与密码逻辑
- `src/beecount/`：BeeCount 兼容与财务集成领域
- `src/mail/`：邮件协议、解析、凭据与服务
- `src/repository/`：Sync 仓库抽象、PostgreSQL 生产实现与仅用于测试/协议 harness 的内存实现
- `src/sync/`：游标、分页令牌、哈希等同步基础设施
- `src/bin/`：独立 worker / admin / migration 二进制
- `crates/lifetrace-contracts/`：共享协议与领域契约
- `crates/lifetrace-sync-client/`：Rust Sync v1 客户端库
- `contracts/`：由 contract exporter 生成的跨语言契约产物
- `migrations/`：PostgreSQL migration
- `deploy/cloud/`：Docker Compose、Caddy 与生产部署示例
- `openspec/`：当前有效规范与变更记录
- `tests/`：服务端集成测试

## 核心 API

稳定的公共能力按以下前缀组织：

| 前缀 | 作用 |
| --- | --- |
| `/health/*` | liveness / readiness |
| `/api/v1/meta/*` | 版本与元信息 |
| `/api/v1/auth/*` | 认证与账号能力 |
| `/api/v1/sync/*` | Push / Pull / Snapshot / capabilities |
| `/api/v1/files/*` | 文件元数据、上传/下载签名 |
| `/api/v1/privacy/*` | 导出、保留策略与账号删除 |
| `/api/v1/integrations/beecount/*` | LifeTrace 对 BeeCount 的只读集成 |
| BeeCount compatibility routes | BeeCount 原生客户端兼容接口 |
| mail routes | 邮件列表、消息与附件 |
| photo routes | 临时照片中转与挑战能力 |

Sync v1 的具体对象类型和字段以 `lifetrace-contracts` 与生成后的 `contracts/` 为准，不在 README 重复维护第二份协议定义。

## 本地运行

应用启动要求提供 `DATABASE_URL`：

```bash
export DATABASE_URL=postgres://lifetrace:password@127.0.0.1:5432/lifetrace
cargo run
```

无数据库的内存仓库只通过测试/协议 harness 直接构造 `AppState` 使用，不是可启动的 Cloud 运行模式。

常用数据库工具：

```bash
cargo run --bin lifetrace-migrate
cargo run --bin lifetrace-admin
```

## 测试与质量门禁

CI 的核心检查可在本地复现：

```bash
cargo fmt --check
cargo test --locked -- --test-threads=1
cargo clippy --locked --all-targets -- -D warnings
cargo test --manifest-path crates/lifetrace-contracts/Cargo.toml
cargo test --manifest-path crates/lifetrace-sync-client/Cargo.toml
cargo run --manifest-path tools/contract-exporter/Cargo.toml
```

生成契约后，`contracts/` 必须保持无未提交差异。

## Docker 与部署

根目录 `Dockerfile` 构建 Cloud 镜像。部署样例位于 `deploy/cloud/`：

- `docker-compose.local.yml`：本地开发
- `docker-compose.test.yml`：测试
- `docker-compose.production.yml`：生产 Compose
- `Caddyfile.production`：生产反向代理
- `deploy-production.sh`：生产部署脚本

生产环境至少需要 PostgreSQL、显式 HTTPS Origin、强随机认证密钥/pepper，并应关闭运行时自动 migration，使用独立 migration 身份执行数据库升级。

## 对象存储

长期文件使用 `file_objects` 元数据 + S3 兼容对象存储。Cloud 只签发短时上传/下载 URL，不代理大文件字节；`file.metadata` 继续通过 Sync v1 跨端同步。

主要配置包括：

```text
FILE_OBJECT_STORAGE_ENDPOINT
FILE_OBJECT_STORAGE_BUCKET
FILE_OBJECT_STORAGE_REGION
FILE_OBJECT_STORAGE_ACCESS_KEY_ID
FILE_OBJECT_STORAGE_SECRET_ACCESS_KEY
FILE_OBJECT_STORAGE_PRESIGN_TTL_SECONDS
FILE_MAX_UPLOAD_BYTES
```

私密本地加密数据不得通过普通文件服务上传。

## 兼容性约束

本仓库正在逐步收拢历史模块，但重构遵循以下边界：

1. 不改变已发布的 Sync v1 wire contract。
2. 不在纯目录重构中修改数据库 schema。
3. BeeCount 兼容接口在替换前保持现有路径和字段语义。
4. 生成契约只由 `lifetrace-contracts` 派生，避免手工维护重复 schema。
5. 生产业务使用 PostgreSQL；内存实现只服务测试与协议 harness。
