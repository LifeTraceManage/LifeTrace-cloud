# Entity Link Scope Verification

This file records final verification evidence for `add-entity-link-scopes-v1` before archive.

## Requirement coverage

- `links:read` and `links:write` are supported authorization scopes.
- Generic `entity.link` resolves to dedicated link scopes rather than account scopes.
- `lifetrace-assets` can receive link read/write while remaining ineligible for `account:write`, `finance:write`, `notes:write`, `execution:write`, and `mail:write`.
- `identity.user` remains protected by `account:write`.
- The existing typed EntityLink DTO, registry entry, Sync v1 persistence, and generated contracts remain unchanged.
- Mainline Execute session scopes introduced after the original EntityLink branch were preserved during reconciliation.

## Exact-head verification

Cloud PR #5 exact head `b3682012b7e88fbb7df97cb7aa49784f6a2a9eec` passed Cloud CI run `35179402544` before merge. The gate covered:

1. OpenSpec strict validation.
2. `cargo fmt --check`.
3. Full locked Rust tests.
4. `cargo clippy --locked --all-targets -- -D warnings`.
5. Contract crate tests.
6. Sync-client crate tests.
7. Deterministic contract regeneration and drift check.
8. Docker image build.

## Merge and post-merge verification

PR #5 merged to `main` as `8b5a10ae569c3153410dcbee9fff703889457dec`.

Post-merge verification on that exact commit passed:

- Cloud CI run `35179961884`: success.
- Cloud Container Image run `35179962046`: success.

The change is therefore eligible for archive and synchronization into the live OpenSpec specification set.
