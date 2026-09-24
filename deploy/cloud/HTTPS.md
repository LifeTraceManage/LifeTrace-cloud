# LifeTrace production HTTPS

Production traffic is terminated by Caddy:

```text
Internet :80/:443
    -> Caddy (automatic TLS + HTTP->HTTPS)
    -> Web/Nginx :80
    -> LifeTrace Cloud :8787
```

## Required DNS

The value of `LIFETRACE_DOMAIN` must resolve to the server public IP. For the current deployment:

```text
lifetrace.store  A  43.142.88.57
```

Do not expose the Web container's port 80 directly. Caddy owns host ports 80 and 443.

## First deployment

```bash
cd ~/LifeTrace-cloud
git pull
cd deploy/cloud

cp .env.production.example .env.production   # only on first setup
nano .env.production
./deploy-production.sh all
```

Set at least:

```env
LIFETRACE_DOMAIN=lifetrace.store
```

Keep the existing production secrets in `.env.production`; never overwrite a configured production file just to adopt the example.

The server/security group must allow inbound TCP 80 and TCP/UDP 443. Caddy stores ACME account and certificates in the persistent `caddy_data` volume and renews certificates automatically.

## Verify

```bash
docker compose --env-file .env.production -f docker-compose.production.yml ps
docker compose --env-file .env.production -f docker-compose.production.yml logs --tail=100 caddy
curl -I http://lifetrace.store
curl -I https://lifetrace.store
```

The HTTP request should redirect to HTTPS and the HTTPS request should return the Web response.
