# Assets Sync v1 Verification

This document records the verification evidence for the active OpenSpec change
`add-assets-sync-v1`.

## Contract and authorization

- `asset.asset` and `asset.event` are registered user-owned bidirectional entities.
- Both payloads are validated through typed Rust DTO dispatch.
- `lifetrace-assets` receives only the required Assets/Sync scopes.
- Asset and AssetEvent both use the shared opaque `ServerVersion` wire type.
- Generated JSON Schema, OpenAPI, and TypeScript artifacts include the Assets domain.

## Automated verification

The Cloud CI gate executes:

1. OpenSpec strict validation.
2. `cargo fmt --check`.
3. Full locked Rust test suite.
4. `cargo clippy --locked --all-targets -- -D warnings`.
5. `lifetrace-contracts` crate tests.
6. `lifetrace-sync-client` crate tests.
7. Deterministic contract regeneration.
8. Generated-contract drift check.
9. Docker image build.

Run `34572077638` passed the complete gate and generated the contract artifacts now committed
on this branch. This final verification commit exists to trigger the same gate on the exact PR
head after generated artifacts were committed by GitHub Actions.

## Archive completion

Exact-head PR CI run `34572759558` passed before merge. The implementation was merged to
`main` as `548d6d2c4d2e86cba25ac1ed42d97ce4c3365faa`, and post-merge Cloud CI runs
`34578955577` and `34578955468` also passed.

The change was archived on 2026-09-11 after its delta specification was synchronized into
`openspec/specs/asset-sync/spec.md`.
