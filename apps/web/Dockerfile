FROM node:22-alpine AS browser-builder
WORKDIR /workspace
COPY package.json ./
RUN npm install --no-audit --no-fund
COPY . .
RUN npm run build

FROM caddy:2.11.4-alpine
COPY Caddyfile.production /etc/caddy/Caddyfile
COPY --from=browser-builder /workspace/dist /srv
COPY photo-challenge-pwa /srv-photo-challenge
EXPOSE 80 8869
