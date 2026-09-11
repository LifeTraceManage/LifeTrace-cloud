# Asset Sync v1 Specification

## Purpose
Defines the LifeTrace Cloud contract for Assets Sync v1, including registered asset entities, typed payload validation, and least-privilege authorization for the dedicated Assets client.

## Requirements

### Requirement: Registered asset entities
The service MUST recognize `asset.asset` and `asset.event` as user-owned bidirectional Sync v1
entities with optimistic conflict handling.

#### Scenario: Asset push
- GIVEN an authenticated Assets client with asset write scope
- WHEN it pushes a valid `asset.asset` payload
- THEN the change SHALL be processed by the normal Sync v1 optimistic version rules

### Requirement: Typed payload validation
The service MUST reject malformed asset payloads before persistence.

#### Scenario: Missing ID
- GIVEN an `asset.asset` upsert without a valid id
- WHEN the change is pushed
- THEN it SHALL be rejected as an invalid entity payload

### Requirement: Least privilege
The dedicated Assets application MUST receive asset scopes without unrelated product write scopes.

#### Scenario: Assets app scopes
- GIVEN the `lifetrace-assets` app id
- WHEN default scopes are calculated
- THEN `assets:read` and `assets:write` SHALL be present
- AND finance/notes/mail write scopes SHALL be absent
