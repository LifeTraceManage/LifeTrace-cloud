# syntax=docker/dockerfile:1

FROM node:22-alpine AS web-builder
WORKDIR /web
COPY apps/web/package.json ./
RUN --mount=type=cache,target=/root/.npm npm install --no-audit --no-fund
COPY apps/web/ ./
RUN npm run typecheck \
    && npm test \
    && npm run build \
    && test -s dist/index.html

FROM nginx:1.27-alpine
COPY apps/web/nginx.conf /etc/nginx/conf.d/default.conf
COPY --from=web-builder /web/dist/ /usr/share/nginx/html/
COPY --from=web-builder /web/photo-challenge-pwa/ /usr/share/nginx/html/photo-challenge-upload/

EXPOSE 80

HEALTHCHECK --interval=30s --timeout=5s --retries=3 \
    CMD wget -q -O - http://127.0.0.1/healthz >/dev/null || exit 1
