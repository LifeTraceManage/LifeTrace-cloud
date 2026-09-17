## Design

The existing generic `entity.link` Sync v1 entity remains unchanged.

Authorization is separated from account mutation:

- identity.user -> account:read/write
- entity.link -> links:read/write

`links:read` and `links:write` are added to the global supported scope catalog.

The Assets app gains only:
- links:read
- links:write

It still does not gain account:write, finance:*, notes:*, execution:*, or mail:*.

Desktop/Web inherit link scopes through ALL_SCOPES.

No database migration is required because scopes are string-based and application authorization is
derived by the current server scope policy.
