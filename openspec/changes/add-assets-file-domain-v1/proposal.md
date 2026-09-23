## Why

LifeTrace Assets needs real photos, invoices, warranty documents, manuals, and other binary attachments.
LifeTrace Cloud already provides the generic file metadata and signed object-storage transfer platform,
but the current file domains do not include Assets and generic `files:read` / `files:write` alone are too
broad as an application authorization boundary.

The Cloud must expose an Assets-specific file domain while preventing the dedicated Assets client from
reading or mutating unrelated Finance, Notes, backup, photo, or other product file domains.

## What Changes

- Add `assets_attachments` to the supported file-domain catalog.
- Allow file ownership references only to `asset.asset` or `asset.event` for this domain.
- Add server-side application/domain authorization so `lifetrace-assets` file operations are limited to `assets_attachments`.
- Permit the Assets client to obtain the file read/write capability required for the generic Files API without granting cross-domain access.
- Apply the domain restriction consistently to list, get, prepare, upload-state, download-url, delete, and orphan-related operations.
- Add authorization and validation regression tests.

## In Scope

Assets file domain registration, owner validation, dedicated application/domain authorization, file-scope
issuance for the Assets client, and regression coverage across the existing Files API.

## Out of Scope

A new file service, changes to object-storage providers, public sharing, OCR/AI processing, Assets UI,
client-side local persistence, and changes to unrelated file-domain authorization behavior unless needed
to centralize the policy safely.

## Rollback

The change is additive. Removing the Assets domain/client policy disables Assets binary synchronization
without changing existing file domains or core Sync v1 entities.
