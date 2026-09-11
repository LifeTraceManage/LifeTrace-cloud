## Why

LifeTrace Assets has become a local-first application and now emits durable outbox operations for
`asset.asset` and `asset.event`. LifeTrace Cloud Sync v1 currently rejects those entity types
because they are absent from the registry and authorization map.

## What Changes

- Register `asset.asset` and `asset.event` as user-owned, bidirectional, optimistic entities.
- Add typed wire payload validation for assets and lifecycle events.
- Add the `lifetrace-assets` application id.
- Add least-privilege `assets:read` and `assets:write` scopes.
- Add registry, payload, authorization, and regression tests.
- Continue to use the existing generic `sync_entities` and `sync_change_log` persistence.

## Out of Scope

Asset-specific REST CRUD endpoints, price scraping, and Finance-side transaction generation.
