# LifeTrace Web

LifeTrace Web 的正式源码位于 `LifeTrace-cloud/apps/web`，与 Rust Cloud 使用同一个 Git 仓库维护，但生产镜像独立构建。

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

Web 使用独立镜像：

```text
ghcr.io/lifetracemanage/lifetrace-web:main
ghcr.io/lifetracemanage/lifetrace-web:sha-<commit>
```

`apps/web/Dockerfile` 使用 Node 22 构建 Vite SPA，再复制到 Nginx Alpine。Nginx：

- 提供 SPA 静态文件；
- 提供 `/photo-challenge-upload`；
- 将 `/api/*` 与 `/health/*` 反向代理到 Compose 中的 `cloud:8787`；
- 支持 WebSocket upgrade；
- 将请求体上限设置为 256 MiB，避免附件上传被默认 1 MiB 限制阻断。

Cloud 独立镜像：

```text
ghcr.io/lifetracemanage/lifetrace-cloud:main
```

两个镜像可以独立构建、升级、回滚。
