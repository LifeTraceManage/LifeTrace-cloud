## Design

Assets uses the existing Sync v1 generic store. No asset-specific PostgreSQL table is introduced.

### Entity types
- `asset.asset`
- `asset.event`

Both are user-owned, bidirectional, schemaVersion 1, optimistic-conflict entities.

### Authentication
The dedicated app id is `lifetrace-assets`. Its default/grantable product scopes are:
- account:read
- devices:read
- sync:read
- sync:write
- assets:read
- assets:write

No finance, notes, execution, or mail scope is implicitly granted.

### Validation
Typed Rust DTOs mirror the Flutter wire payload. Sync processing validates payload shape and entity
ID before storing it in the generic sync tables.

### Compatibility
The change is additive. Existing entity descriptors, cursor format, conflict rules, and storage
schema remain unchanged.
