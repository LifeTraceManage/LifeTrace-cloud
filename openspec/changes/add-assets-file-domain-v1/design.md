## Context

The Cloud file platform already supports metadata preparation, SHA-256 deduplication, signed upload and
download URLs, completion/failure state, deletion, orphan inspection, and object-storage integration.
Supported file domains are currently enumerated centrally, and endpoint authorization relies primarily
on generic `files:read` / `files:write` scopes.

That scope boundary is sufficient for trusted broad clients but is too coarse for the dedicated
`lifetrace-assets` application: granting generic file scopes without a second server-side restriction
would allow the app to address unrelated file domains.

## Design

### Assets file domain

Add `assets_attachments` as a first-class file domain. Preparation requests in this domain MUST carry
an owner reference with:

- `entityType = asset.asset` or `asset.event`
- non-empty `entityId`

Other owner entity types are rejected before creating a file object.

### Application/domain authorization

Introduce a centralized file-domain authorization policy that receives the authenticated principal,
requested operation, and target domain/file metadata.

For the dedicated `lifetrace-assets` application:

- read operations are permitted only for `assets_attachments`
- write operations are permitted only for `assets_attachments`
- any attempt to address another domain is rejected regardless of generic file scopes

The generic `files:read` / `files:write` scopes remain necessary but are not sufficient for this app.
This preserves compatibility with the existing Files API while enforcing least privilege server-side.

Existing applications retain their current behavior unless an explicit domain policy already exists or
is introduced separately. This change MUST NOT silently narrow unrelated clients.

### Endpoint coverage

The policy must be applied consistently to every operation that can expose or mutate file metadata or
binary access:

- list files
- get file metadata
- prepare/create file
- generate upload URL if exposed separately
- complete upload
- mark upload failed
- generate download URL
- delete file
- orphan queries/actions where a domain or file is visible

For ID-addressed routes, the server resolves the file record first and checks its domain before returning
metadata, signed URLs, or mutation success.

### Scope issuance

Allow the registered `lifetrace-assets` client to request the file read/write scopes needed by the Files
API. Domain isolation must not depend on the client omitting other domain names; it is enforced after
authentication on every request.

### Validation and MIME policy

`assets_attachments` uses the existing file-size limit and object-storage lifecycle. MIME validation
should support the V1 client policy for common images, PDF, text, and common office documents while
continuing to reject dangerous or unsupported types according to the existing platform policy.

### Observability and errors

Cross-domain attempts should return authorization failure without revealing existence-sensitive metadata
for inaccessible files. Invalid owner type is a request-validation error. Existing logging/tracing should
include the domain and operation without logging signed URLs or binary content.

## Testing

Regression coverage must prove:

- `assets_attachments` is accepted by file preparation
- valid `asset.asset` and `asset.event` owners are accepted
- unsupported owner types are rejected
- `lifetrace-assets` can list/prepare/download/delete Assets-domain files with correct scopes
- the same client is rejected for every tested unrelated domain
- ID-addressed access to an unrelated-domain file is rejected after lookup
- missing file scopes still fail even for the allowed Assets domain
- existing non-Assets file-domain behavior remains unchanged
- existing object-storage and PostgreSQL file tests remain green

## Non-Goals

No new object-storage provider, no public file ACLs, no binary transport through Sync v1, and no
application-level attachment model are introduced in this Cloud change.
