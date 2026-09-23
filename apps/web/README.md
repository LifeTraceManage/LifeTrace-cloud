# LifeTrace Web

LifeTrace Web 的正式源码位于 `LifeTrace-cloud/apps/web`，与 Rust Cloud 作为一个 monorepo 维护。

## Development

```bash
cd apps/web
npm install
npm run dev
```

质量检查：

```bash
npm run typecheck
npm test
npm run build
```

## Production

生产环境不再单独部署 Web 容器，也不再使用 Caddy。仓库根目录 `Dockerfile` 会：

1. 使用 Node 22 构建 `apps/web`；
2. 执行 Web typecheck、tests 与 Vite production build；
3. 编译 Rust `lifetrace-cloud`；
4. 将 `dist/` 复制到最终镜像 `/app/web`；
5. 由 Axum 同源提供 Web 与 `/api/v1/*`。

GitHub Actions 发布完整制品：

```text
ghcr.io/lifetracemanage/lifetrace-cloud:main
ghcr.io/lifetracemanage/lifetrace-cloud:sha-<commit>
```

独立的 `LifeTrace-web` 仓库只保留历史，不再作为生产源码来源。
