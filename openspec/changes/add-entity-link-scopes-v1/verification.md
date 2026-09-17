# Entity Link Scope Verification

This file records verification evidence for the active OpenSpec change
`add-entity-link-scopes-v1`.

## Requirement coverage

- `links:read` and `links:write` are supported authorization scopes.
- Generic `entity.link` now resolves to the dedicated link scopes rather than account scopes.
- `lifetrace-assets` can receive link read/write while remaining ineligible for
  `account:write`, `finance:write`, `notes:write`, `execution:write`, and `mail:write`.
- `identity.user` remains protected by `account:write`.
- The existing typed EntityLink DTO, registry entry, Sync v1 persistence, and generated contracts
  remain unchanged.

## Automated verification

Cloud PR #5 run `34618971886` passed the complete gate on
`7d6857b1ddc46bf59645c3dc8f7fd60922bcc95f`:

1. OpenSpec `validate --all --strict --no-interactive`.
2. `cargo fmt --check`.
3. Full locked Rust tests.
4. `cargo clippy --locked --all-targets -- -D warnings`.
5. Contract crate tests.
6. Sync-client crate tests.
7. Deterministic contract regeneration and drift check.
8. Docker image build.

This verification commit intentionally triggers the same complete gate again so the PR is merged
only from a clean exact head.

## Merge and archive gate

The implementation must be merged only after this verification head is green. The OpenSpec change
must be archived only after post-merge `main` verification succeeds.
