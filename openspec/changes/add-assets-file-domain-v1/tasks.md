## 1. Domain and owner validation

- [ ] 1.1 Add `assets_attachments` to the supported file-domain catalog.
- [ ] 1.2 Add owner validation allowing only `asset.asset` and `asset.event` for this domain.
- [ ] 1.3 Add MIME-policy coverage required by the Assets V1 attachment spec without weakening existing safety checks.

## 2. Authorization

- [ ] 2.1 Add a centralized file-domain authorization helper/policy.
- [ ] 2.2 Restrict authenticated `lifetrace-assets` principals to `assets_attachments`.
- [ ] 2.3 Apply the policy to domain-addressed list/prepare/orphan operations.
- [ ] 2.4 Apply the policy after file lookup to ID-addressed get/upload/complete/fail/download/delete operations.
- [ ] 2.5 Keep existing generic `files:read` / `files:write` scope enforcement in place.
- [ ] 2.6 Allow the registered `lifetrace-assets` client to request the required file scopes.

## 3. Tests

- [ ] 3.1 Add failing authorization tests for cross-domain Assets access before implementation.
- [ ] 3.2 Add positive tests for valid `asset.asset` / `asset.event` attachment operations.
- [ ] 3.3 Add negative owner-type validation tests.
- [ ] 3.4 Add ID-addressed cross-domain regression tests for metadata, signed URLs, state changes, and deletion.
- [ ] 3.5 Add missing-scope tests proving domain permission does not bypass file scopes.
- [ ] 3.6 Add compatibility regression tests for existing non-Assets domains/clients.

## 4. Verification and release

- [ ] 4.1 `cargo fmt --all -- --check` passes.
- [ ] 4.2 Cloud unit/integration tests pass, including PostgreSQL file tests.
- [ ] 4.3 `cargo clippy --workspace --all-targets --all-features -- -D warnings` passes.
- [ ] 4.4 Contract and generated-drift checks pass.
- [ ] 4.5 OpenSpec strict validation passes.
- [ ] 4.6 Container build passes.
- [ ] 4.7 Merge to `main`, verify post-merge CI/container workflow, then archive this OpenSpec change and publish the live spec.
