# Delta for Entity Link Authorization

## ADDED Requirements

### Requirement: Dedicated link scopes
The Cloud MUST authorize generic `entity.link` synchronization through dedicated link scopes
rather than account write authority.

#### Scenario: Read link
- GIVEN a principal with links:read and sync:read
- WHEN entity.link is requested through Sync v1
- THEN the read SHALL be authorized without account:read being used as the entity-link gate

#### Scenario: Write link
- GIVEN a principal with links:write and sync:write
- WHEN a valid entity.link mutation is pushed
- THEN the write SHALL be authorized without account:write

### Requirement: Assets least privilege
The dedicated Assets application MUST be eligible for link read/write scopes while remaining
ineligible for account write and unrelated product write scopes.

#### Scenario: Assets grants
- GIVEN the lifetrace-assets app id
- WHEN allowed scopes are calculated
- THEN links:read and links:write SHALL be present
- AND account:write, finance:write, notes:write, execution:write, and mail:write SHALL be absent

### Requirement: Identity authorization unchanged
Identity user entities MUST remain protected by account scopes.

#### Scenario: Identity write
- GIVEN an identity.user mutation
- WHEN its required write scope is calculated
- THEN account:write SHALL still be required
