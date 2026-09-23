# Delta for Assets File Domain

## ADDED Requirements

### Requirement: Assets attachment file domain
LifeTrace Cloud MUST recognize `assets_attachments` as a supported file domain for the generic Files API.

#### Scenario: Prepare Assets attachment
- GIVEN a request with valid file metadata
- AND `domain = assets_attachments`
- AND a supported Assets owner reference
- WHEN file preparation runs
- THEN the domain SHALL be accepted
- AND normal file metadata/deduplication behavior SHALL apply

### Requirement: Supported Assets owners
Files in `assets_attachments` MUST be owned only by `asset.asset` or `asset.event` entities.

#### Scenario: Asset owner
- GIVEN `entityType = asset.asset` and a non-empty entity ID
- WHEN an Assets attachment is prepared
- THEN the owner reference SHALL be accepted

#### Scenario: Event owner
- GIVEN `entityType = asset.event` and a non-empty entity ID
- WHEN an Assets attachment is prepared
- THEN the owner reference SHALL be accepted

#### Scenario: Unrelated owner
- GIVEN `domain = assets_attachments`
- AND `entityType` is not `asset.asset` or `asset.event`
- WHEN preparation is attempted
- THEN Cloud SHALL reject the request before creating a file object

### Requirement: Assets application domain isolation
The authenticated `lifetrace-assets` application MUST be restricted server-side to the
`assets_attachments` domain for Files API operations.

#### Scenario: Allowed Assets-domain read
- GIVEN a `lifetrace-assets` principal with the required file read scope
- WHEN it lists or retrieves an `assets_attachments` file
- THEN the operation SHALL be allowed subject to normal ownership rules

#### Scenario: Allowed Assets-domain write
- GIVEN a `lifetrace-assets` principal with the required file write scope
- WHEN it prepares, completes, fails, or deletes an `assets_attachments` file
- THEN the operation SHALL be allowed subject to normal validation

#### Scenario: Cross-domain list attempt
- GIVEN a `lifetrace-assets` principal with generic file read scope
- WHEN it requests another product file domain
- THEN Cloud SHALL reject the operation

#### Scenario: Cross-domain file-ID attempt
- GIVEN a `lifetrace-assets` principal
- AND a file ID belonging to another product domain
- WHEN it requests metadata, a signed download/upload action, state mutation, or delete by ID
- THEN Cloud SHALL reject the operation without granting access to that file

### Requirement: Generic file scopes remain necessary
Assets-domain authorization MUST NOT bypass existing `files:read` and `files:write` scope checks.

#### Scenario: Missing read scope
- GIVEN a `lifetrace-assets` principal without file read permission
- WHEN it requests an `assets_attachments` read operation
- THEN Cloud SHALL reject the request

#### Scenario: Missing write scope
- GIVEN a `lifetrace-assets` principal without file write permission
- WHEN it requests an `assets_attachments` write operation
- THEN Cloud SHALL reject the request

### Requirement: Assets client may obtain required file scopes
The registered `lifetrace-assets` OAuth/application configuration MUST allow requesting the generic file
read/write scopes required by the Files API while relying on server-side domain isolation for least privilege.

#### Scenario: Assets authorization request
- GIVEN the registered `lifetrace-assets` client
- WHEN it requests its approved Assets, link, and file scopes
- THEN the file scopes required for attachment synchronization SHALL be issuable
- AND unrelated account/product scopes SHALL not become required merely for file attachments

### Requirement: Endpoint-complete domain enforcement
Assets application domain isolation MUST apply to all Files API paths that can reveal or mutate file
metadata or signed binary-transfer access.

#### Scenario: Signed download URL
- GIVEN a `lifetrace-assets` principal
- WHEN it requests a download URL for an allowed Assets-domain file
- THEN normal signed-URL generation MAY proceed
- BUT the same operation for an unrelated-domain file SHALL be rejected

#### Scenario: Orphan query
- GIVEN a `lifetrace-assets` principal
- WHEN it invokes an orphan/file-maintenance endpoint
- THEN results and actions SHALL be restricted to `assets_attachments`
- AND unrelated-domain metadata SHALL not be exposed

### Requirement: Existing file-domain compatibility
Introducing Assets domain isolation MUST NOT unintentionally change authorized behavior for existing
non-Assets clients and domains.

#### Scenario: Existing non-Assets client
- GIVEN an existing authorized client using a pre-existing file domain
- WHEN it performs an operation that was valid before this change
- THEN the operation SHALL remain valid unless an explicit pre-existing policy already restricts it
