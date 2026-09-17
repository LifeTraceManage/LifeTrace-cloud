## Why

LifeTrace Cloud already exposes a typed generic `entity.link` entity, but authorization maps it to
`account:read/write`. The Assets app intentionally does not receive `account:write`, because
that scope also protects identity/account mutations.

Cross-application references therefore need a dedicated least-privilege scope boundary.

## What Changes

- Add `links:read` and `links:write` authorization scopes.
- Map `entity.link` to the new link scopes instead of account scopes.
- Allow the dedicated `lifetrace-assets` app to request link read/write scopes.
- Keep identity.user protected by account scopes.
- Add authorization regression tests.

## Out of Scope

Changing the EntityLink DTO, adding graph traversal APIs, resolving target entities, or granting
Assets unrelated Finance/Execution/Notes scopes.
