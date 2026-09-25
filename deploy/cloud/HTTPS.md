# LifeTrace HTTPS and ICP transition access

The production gateway keeps two entry points during the domain/ICP transition:

```text
http://43.142.88.57
    -> Caddy :80
    -> Web/Nginx :80
    -> LifeTrace Cloud :8787

https://lifetrace.store
    -> Caddy :443 (automatic TLS once DNS is reachable)
    -> Web/Nginx :80
    -> LifeTrace Cloud :8787
```

The IP route allows LifeTrace to remain usable while the domain is waiting for DNS/ICP availability.

## Environment

Keep the existing secrets in `.env.production` and add/update:

```env
LIFETRACE_DOMAIN=lifetrace.store
LIFETRACE_PUBLIC_IP=43.142.88.57
PUBLIC_WEB_BASE_URL=https://lifetrace.store
CORS_ALLOWED_ORIGINS=https://lifetrace.store,http://43.142.88.57
AUTH_COOKIE_SECURE=false
```

`AUTH_COOKIE_SECURE=false` is intentionally temporary. A browser will not send a Secure session cookie over the plain HTTP IP endpoint. Once `https://lifetrace.store` is fully available, switch it to:

```env
CORS_ALLOWED_ORIGINS=https://lifetrace.store
AUTH_COOKIE_SECURE=true
```

and stop using the HTTP IP endpoint for authenticated access.

## Deploy

```bash
cd ~/LifeTrace-cloud
git pull
cd deploy/cloud
./deploy-production.sh all
```

The security group must allow TCP 80. TCP 443 is required for domain HTTPS once DNS is available. UDP 443 is optional for HTTP/3.

## Verify during the transition

```bash
curl -I http://43.142.88.57
docker compose --env-file .env.production -f docker-compose.production.yml ps
docker compose --env-file .env.production -f docker-compose.production.yml logs --tail=100 caddy
```

After DNS is available:

```bash
curl -I http://lifetrace.store
curl -I https://lifetrace.store
```

Caddy will keep retrying certificate acquisition if the domain is not yet publicly reachable. That does not prevent the explicit HTTP IP site from serving traffic.
