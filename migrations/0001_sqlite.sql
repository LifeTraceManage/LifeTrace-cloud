CREATE TABLE cloud_users (
    id TEXT PRIMARY KEY,
    status TEXT NOT NULL DEFAULT 'active' CHECK (status IN ('active','disabled')),
    email TEXT,
    email_normalized TEXT UNIQUE,
    display_name TEXT,
    password_hash TEXT,
    password_version INTEGER NOT NULL DEFAULT 1,
    email_verified_at TEXT,
    password_changed_at TEXT,
    disabled_at TEXT,
    registration_source TEXT,
    failed_login_count INTEGER NOT NULL DEFAULT 0,
    locked_until TEXT,
    auth_state TEXT NOT NULL DEFAULT 'pending'
        CHECK (auth_state IN ('pending','active','password_reset_required','disabled')),
    created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE TABLE cloud_devices (
    id TEXT PRIMARY KEY,
    user_id TEXT NOT NULL REFERENCES cloud_users(id) ON DELETE CASCADE,
    app_id TEXT NOT NULL,
    platform TEXT NOT NULL,
    client_version TEXT,
    protocol_version INTEGER,
    schema_version INTEGER,
    status TEXT NOT NULL DEFAULT 'active' CHECK (status IN ('active','revoked')),
    external_device_id TEXT NOT NULL,
    device_group_id TEXT,
    device_name TEXT,
    os_version TEXT,
    device_model TEXT,
    first_seen_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
    last_seen_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
    last_sync_at TEXT,
    last_login_at TEXT,
    last_login_ip TEXT,
    last_user_agent TEXT,
    revoked_at TEXT,
    revoked_reason TEXT
);
CREATE UNIQUE INDEX idx_cloud_devices_external_identity ON cloud_devices(user_id,app_id,external_device_id);
CREATE INDEX idx_cloud_devices_user_status ON cloud_devices(user_id,status,last_seen_at DESC);

CREATE TABLE sync_entities (
    user_id TEXT NOT NULL REFERENCES cloud_users(id) ON DELETE CASCADE,
    entity_type TEXT NOT NULL,
    entity_id TEXT NOT NULL,
    entity_schema_version INTEGER NOT NULL,
    server_version INTEGER NOT NULL CHECK (server_version > 0),
    payload TEXT,
    payload_hash BLOB,
    is_deleted INTEGER NOT NULL DEFAULT 0,
    deleted_at TEXT,
    origin_device_id TEXT,
    origin_device_external_id TEXT,
    created_at TEXT NOT NULL,
    server_modified_at TEXT NOT NULL,
    client_modified_at TEXT,
    last_cursor INTEGER NOT NULL,
    PRIMARY KEY(user_id,entity_type,entity_id)
);
CREATE INDEX idx_sync_entities_user_type ON sync_entities(user_id,entity_type);
CREATE INDEX idx_sync_entities_user_active ON sync_entities(user_id,is_deleted,entity_type);
CREATE INDEX idx_sync_entities_last_cursor ON sync_entities(user_id,last_cursor);

CREATE TABLE sync_processed_changes (
    user_id TEXT NOT NULL REFERENCES cloud_users(id) ON DELETE CASCADE,
    change_id TEXT NOT NULL,
    request_hash BLOB NOT NULL,
    result_status TEXT NOT NULL,
    result_json TEXT NOT NULL,
    entity_type TEXT NOT NULL,
    entity_id TEXT NOT NULL,
    processed_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
    PRIMARY KEY(user_id,change_id)
);

CREATE TABLE sync_change_log (
    cursor INTEGER PRIMARY KEY AUTOINCREMENT,
    user_id TEXT NOT NULL REFERENCES cloud_users(id) ON DELETE CASCADE,
    entity_type TEXT NOT NULL,
    entity_id TEXT NOT NULL,
    operation TEXT NOT NULL CHECK (operation IN ('upsert','delete')),
    entity_schema_version INTEGER NOT NULL,
    server_version INTEGER NOT NULL,
    payload TEXT,
    payload_hash BLOB,
    tombstone TEXT,
    origin_device_id TEXT,
    origin_device_external_id TEXT,
    client_modified_at TEXT,
    server_modified_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
);
CREATE INDEX idx_change_log_user_cursor ON sync_change_log(user_id,cursor);
CREATE INDEX idx_change_log_user_type_cursor ON sync_change_log(user_id,entity_type,cursor);

CREATE TABLE sync_snapshots (
    id TEXT PRIMARY KEY,
    user_id TEXT NOT NULL REFERENCES cloud_users(id) ON DELETE CASCADE,
    scope_hash BLOB NOT NULL,
    snapshot_cursor INTEGER NOT NULL,
    status TEXT NOT NULL CHECK (status IN ('building','ready','failed','expired')),
    item_count INTEGER NOT NULL DEFAULT 0,
    created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
    expires_at TEXT NOT NULL,
    completed_at TEXT,
    error_message TEXT
);
CREATE TABLE sync_snapshot_items (
    snapshot_id TEXT NOT NULL REFERENCES sync_snapshots(id) ON DELETE CASCADE,
    entity_type TEXT NOT NULL,
    entity_id TEXT NOT NULL,
    entity_schema_version INTEGER NOT NULL,
    server_version INTEGER NOT NULL,
    payload TEXT NOT NULL,
    payload_hash BLOB NOT NULL,
    server_modified_at TEXT NOT NULL,
    PRIMARY KEY(snapshot_id,entity_type,entity_id)
);
CREATE INDEX idx_snapshot_items_keyset ON sync_snapshot_items(snapshot_id,entity_type,entity_id);

CREATE TABLE auth_app_grants (
    id TEXT PRIMARY KEY,
    user_id TEXT NOT NULL REFERENCES cloud_users(id) ON DELETE CASCADE,
    app_id TEXT NOT NULL,
    scopes TEXT NOT NULL DEFAULT '[]',
    status TEXT NOT NULL DEFAULT 'active' CHECK (status IN ('active','revoked')),
    granted_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
    revoked_at TEXT,
    updated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
    UNIQUE(user_id,app_id)
);
CREATE TABLE auth_sessions (
    id TEXT PRIMARY KEY,
    user_id TEXT NOT NULL REFERENCES cloud_users(id) ON DELETE CASCADE,
    device_id TEXT NOT NULL REFERENCES cloud_devices(id) ON DELETE CASCADE,
    app_id TEXT NOT NULL,
    scopes TEXT NOT NULL DEFAULT '[]',
    session_type TEXT NOT NULL CHECK (session_type IN ('native','web')),
    status TEXT NOT NULL DEFAULT 'active' CHECK (status IN ('active','revoked','expired')),
    created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
    last_seen_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
    idle_expires_at TEXT NOT NULL,
    absolute_expires_at TEXT NOT NULL,
    revoked_at TEXT,
    revoked_reason TEXT,
    login_ip TEXT,
    last_ip TEXT,
    user_agent TEXT,
    public_device INTEGER NOT NULL DEFAULT 0
);
CREATE TABLE auth_access_tokens (
    id TEXT PRIMARY KEY,
    session_id TEXT NOT NULL REFERENCES auth_sessions(id) ON DELETE CASCADE,
    token_hash BLOB NOT NULL UNIQUE,
    scopes TEXT NOT NULL DEFAULT '[]',
    created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
    expires_at TEXT NOT NULL,
    revoked_at TEXT,
    revoked_reason TEXT,
    last_used_at TEXT
);
CREATE TABLE auth_refresh_tokens (
    id TEXT PRIMARY KEY,
    session_id TEXT NOT NULL REFERENCES auth_sessions(id) ON DELETE CASCADE,
    family_id TEXT NOT NULL,
    parent_token_id TEXT REFERENCES auth_refresh_tokens(id),
    replaced_by_token_id TEXT REFERENCES auth_refresh_tokens(id),
    token_hash BLOB NOT NULL UNIQUE,
    created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
    idle_expires_at TEXT NOT NULL,
    absolute_expires_at TEXT NOT NULL,
    used_at TEXT,
    revoked_at TEXT,
    revoked_reason TEXT,
    reuse_detected_at TEXT
);
CREATE TABLE auth_web_sessions (
    id TEXT PRIMARY KEY,
    session_id TEXT NOT NULL UNIQUE REFERENCES auth_sessions(id) ON DELETE CASCADE,
    token_hash BLOB NOT NULL UNIQUE,
    csrf_hash BLOB NOT NULL,
    created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
    rotated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
    expires_at TEXT NOT NULL,
    revoked_at TEXT
);
CREATE TABLE auth_password_reset_tokens (
    id TEXT PRIMARY KEY,
    user_id TEXT NOT NULL REFERENCES cloud_users(id) ON DELETE CASCADE,
    token_hash BLOB NOT NULL UNIQUE,
    created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
    expires_at TEXT NOT NULL,
    used_at TEXT,
    revoked_at TEXT,
    requested_ip TEXT
);
CREATE TABLE auth_registration_invites (
    id TEXT PRIMARY KEY,
    token_hash BLOB NOT NULL UNIQUE,
    email_normalized TEXT,
    created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
    expires_at TEXT NOT NULL,
    used_at TEXT,
    revoked_at TEXT,
    created_by TEXT REFERENCES cloud_users(id)
);
CREATE TABLE auth_login_attempts (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    email_hash BLOB NOT NULL,
    ip_address TEXT,
    succeeded INTEGER NOT NULL,
    failure_reason TEXT,
    attempted_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
);
CREATE TABLE auth_audit_log (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    user_id TEXT REFERENCES cloud_users(id) ON DELETE SET NULL,
    session_id TEXT REFERENCES auth_sessions(id) ON DELETE SET NULL,
    device_id TEXT REFERENCES cloud_devices(id) ON DELETE SET NULL,
    app_id TEXT,
    event_type TEXT NOT NULL,
    outcome TEXT NOT NULL,
    ip_address TEXT,
    user_agent TEXT,
    metadata TEXT NOT NULL DEFAULT '{}',
    created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
);
CREATE INDEX idx_auth_sessions_user_status ON auth_sessions(user_id,status,created_at DESC);
CREATE INDEX idx_auth_access_session_active ON auth_access_tokens(session_id,expires_at) WHERE revoked_at IS NULL;
CREATE INDEX idx_auth_refresh_family ON auth_refresh_tokens(family_id);
CREATE INDEX idx_auth_audit_user_time ON auth_audit_log(user_id,created_at DESC);

CREATE TABLE mail_accounts (
    id TEXT PRIMARY KEY,
    user_id TEXT NOT NULL REFERENCES cloud_users(id) ON DELETE CASCADE,
    provider TEXT NOT NULL,
    email_address TEXT NOT NULL,
    display_name TEXT,
    imap_host TEXT NOT NULL,
    imap_port INTEGER NOT NULL,
    imap_security TEXT NOT NULL,
    smtp_host TEXT NOT NULL,
    smtp_port INTEGER NOT NULL,
    smtp_security TEXT NOT NULL,
    username TEXT NOT NULL,
    credential_ciphertext BLOB NOT NULL,
    credential_nonce BLOB NOT NULL,
    status TEXT NOT NULL DEFAULT 'validating',
    idle_supported INTEGER NOT NULL DEFAULT 0,
    last_validated_at TEXT,
    last_sync_at TEXT,
    last_error_code TEXT,
    created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
    deleted_at TEXT
);
CREATE UNIQUE INDEX idx_mail_accounts_user_address_provider ON mail_accounts(user_id,lower(email_address),provider) WHERE deleted_at IS NULL;
CREATE TABLE mail_folders (
    id TEXT PRIMARY KEY,
    user_id TEXT NOT NULL REFERENCES cloud_users(id) ON DELETE CASCADE,
    account_id TEXT NOT NULL REFERENCES mail_accounts(id) ON DELETE CASCADE,
    remote_name TEXT NOT NULL,
    normalized_role TEXT NOT NULL DEFAULT 'other',
    uidvalidity INTEGER, uidnext INTEGER, highest_modseq INTEGER,
    last_seen_uid INTEGER NOT NULL DEFAULT 0,
    last_sync_at TEXT, sync_enabled INTEGER NOT NULL DEFAULT 1,
    created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
    UNIQUE(account_id,remote_name)
);
CREATE TABLE mail_threads (
    id TEXT PRIMARY KEY,
    user_id TEXT NOT NULL REFERENCES cloud_users(id) ON DELETE CASCADE,
    account_id TEXT NOT NULL REFERENCES mail_accounts(id) ON DELETE CASCADE,
    normalized_subject TEXT NOT NULL DEFAULT '',
    latest_message_at TEXT,
    message_count INTEGER NOT NULL DEFAULT 0,
    unread_count INTEGER NOT NULL DEFAULT 0,
    participant_summary TEXT, snippet TEXT,
    created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
);
CREATE TABLE mail_messages (
    id TEXT PRIMARY KEY,
    user_id TEXT NOT NULL REFERENCES cloud_users(id) ON DELETE CASCADE,
    account_id TEXT NOT NULL REFERENCES mail_accounts(id) ON DELETE CASCADE,
    folder_id TEXT NOT NULL REFERENCES mail_folders(id) ON DELETE CASCADE,
    thread_id TEXT NOT NULL REFERENCES mail_threads(id) ON DELETE CASCADE,
    remote_uid INTEGER NOT NULL, uidvalidity INTEGER NOT NULL,
    message_id TEXT, in_reply_to TEXT,
    references_json TEXT NOT NULL DEFAULT '[]',
    subject TEXT NOT NULL DEFAULT '', normalized_subject TEXT NOT NULL DEFAULT '',
    from_json TEXT NOT NULL DEFAULT '[]', to_json TEXT NOT NULL DEFAULT '[]',
    cc_json TEXT NOT NULL DEFAULT '[]', bcc_json TEXT NOT NULL DEFAULT '[]',
    reply_to_json TEXT NOT NULL DEFAULT '[]',
    sent_at TEXT, received_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
    flags_json TEXT NOT NULL DEFAULT '[]',
    is_read INTEGER NOT NULL DEFAULT 0, is_archived INTEGER NOT NULL DEFAULT 0,
    size_bytes INTEGER, snippet TEXT, body_text TEXT, body_html_sanitized TEXT,
    has_attachments INTEGER NOT NULL DEFAULT 0, content_hash TEXT NOT NULL,
    created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
    UNIQUE(account_id,folder_id,uidvalidity,remote_uid)
);
CREATE TABLE mail_attachments (
    id TEXT PRIMARY KEY,
    user_id TEXT NOT NULL REFERENCES cloud_users(id) ON DELETE CASCADE,
    message_id TEXT NOT NULL REFERENCES mail_messages(id) ON DELETE CASCADE,
    part_id TEXT NOT NULL, filename TEXT, mime_type TEXT,
    size_bytes INTEGER NOT NULL DEFAULT 0, content_id TEXT, disposition TEXT,
    checksum TEXT, storage_ref TEXT, download_state TEXT NOT NULL DEFAULT 'metadata_only',
    created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
    UNIQUE(message_id,part_id)
);
CREATE TABLE mail_sync_jobs (
    id TEXT PRIMARY KEY,
    account_id TEXT NOT NULL REFERENCES mail_accounts(id) ON DELETE CASCADE,
    folder_id TEXT REFERENCES mail_folders(id) ON DELETE CASCADE,
    kind TEXT NOT NULL, state TEXT NOT NULL,
    cursor_before_json TEXT, cursor_after_json TEXT,
    attempt INTEGER NOT NULL DEFAULT 0, started_at TEXT, finished_at TEXT,
    next_retry_at TEXT, error_code TEXT, error_detail_redacted TEXT,
    created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
);
CREATE TABLE mail_identities (
    id TEXT PRIMARY KEY,
    user_id TEXT NOT NULL REFERENCES cloud_users(id) ON DELETE CASCADE,
    account_id TEXT NOT NULL REFERENCES mail_accounts(id) ON DELETE CASCADE,
    email_address TEXT NOT NULL,
    display_name TEXT,
    reply_to TEXT,
    signature_html TEXT,
    is_default INTEGER NOT NULL DEFAULT 0,
    created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
    deleted_at TEXT
);
CREATE UNIQUE INDEX idx_mail_identities_account_address
ON mail_identities(account_id, lower(email_address))
WHERE deleted_at IS NULL;
CREATE UNIQUE INDEX idx_mail_identities_account_default
ON mail_identities(account_id)
WHERE is_default=1 AND deleted_at IS NULL;

CREATE TABLE mail_drafts (
    id TEXT PRIMARY KEY,
    user_id TEXT NOT NULL REFERENCES cloud_users(id) ON DELETE CASCADE,
    account_id TEXT NOT NULL REFERENCES mail_accounts(id) ON DELETE CASCADE,
    identity_id TEXT REFERENCES mail_identities(id) ON DELETE SET NULL,
    thread_id TEXT REFERENCES mail_threads(id) ON DELETE SET NULL,
    in_reply_to_message_id TEXT REFERENCES mail_messages(id) ON DELETE SET NULL,
    to_json TEXT NOT NULL DEFAULT '[]', cc_json TEXT NOT NULL DEFAULT '[]',
    bcc_json TEXT NOT NULL DEFAULT '[]', subject TEXT NOT NULL DEFAULT '',
    body_text TEXT NOT NULL DEFAULT '', state TEXT NOT NULL DEFAULT 'draft',
    created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
);
CREATE INDEX idx_mail_drafts_user_updated
ON mail_drafts(user_id, updated_at DESC)
WHERE state='draft';

CREATE TABLE mail_draft_attachments (
    id TEXT PRIMARY KEY,
    user_id TEXT NOT NULL REFERENCES cloud_users(id) ON DELETE CASCADE,
    draft_id TEXT NOT NULL REFERENCES mail_drafts(id) ON DELETE CASCADE,
    filename TEXT NOT NULL,
    mime_type TEXT NOT NULL DEFAULT 'application/octet-stream',
    size_bytes INTEGER NOT NULL CHECK (size_bytes > 0 AND size_bytes <= 18874368),
    content BLOB NOT NULL,
    created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
);
CREATE INDEX idx_mail_draft_attachments_draft
ON mail_draft_attachments(user_id, draft_id, created_at);
CREATE TABLE mail_outbox (
    id TEXT PRIMARY KEY,
    user_id TEXT NOT NULL REFERENCES cloud_users(id) ON DELETE CASCADE,
    account_id TEXT NOT NULL REFERENCES mail_accounts(id) ON DELETE CASCADE,
    draft_id TEXT REFERENCES mail_drafts(id) ON DELETE SET NULL,
    idempotency_key TEXT NOT NULL, state TEXT NOT NULL DEFAULT 'queued',
    generated_message_id TEXT, attempt INTEGER NOT NULL DEFAULT 0,
    next_retry_at TEXT, last_error_code TEXT, sent_at TEXT,
    created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
    UNIQUE(user_id,idempotency_key)
);

CREATE TABLE beecount_identity_links (
    user_id TEXT PRIMARY KEY REFERENCES cloud_users(id) ON DELETE CASCADE,
    beecount_user_id TEXT NOT NULL UNIQUE,
    source_email_normalized TEXT, source_kind TEXT NOT NULL DEFAULT 'native',
    linked_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
    source_created_at TEXT, metadata TEXT NOT NULL DEFAULT '{}'
);
CREATE TABLE beecount_entity_clocks (
    user_id TEXT NOT NULL REFERENCES cloud_users(id) ON DELETE CASCADE,
    entity_type TEXT NOT NULL, entity_sync_id TEXT NOT NULL, ledger_id TEXT,
    scope TEXT NOT NULL, updated_at TEXT NOT NULL, updated_by_device_id TEXT NOT NULL,
    lifetrace_entity_type TEXT NOT NULL, lifetrace_entity_id TEXT NOT NULL,
    lifetrace_server_version INTEGER NOT NULL, lifetrace_cursor INTEGER NOT NULL,
    source_change_id INTEGER, is_deleted INTEGER NOT NULL DEFAULT 0,
    created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
    PRIMARY KEY(user_id,entity_type,entity_sync_id)
);
CREATE UNIQUE INDEX idx_beecount_entity_clocks_lifetrace_entity ON beecount_entity_clocks(user_id,lifetrace_entity_type,lifetrace_entity_id);
CREATE TABLE beecount_migration_runs (
    id TEXT PRIMARY KEY, user_id TEXT NOT NULL REFERENCES cloud_users(id) ON DELETE CASCADE,
    source_fingerprint TEXT NOT NULL, status TEXT NOT NULL DEFAULT 'planned',
    source_cursor INTEGER NOT NULL DEFAULT 0, imported_source_cursor INTEGER NOT NULL DEFAULT 0,
    imported_entities INTEGER NOT NULL DEFAULT 0, imported_tombstones INTEGER NOT NULL DEFAULT 0,
    comparison_mismatches INTEGER NOT NULL DEFAULT 0, started_at TEXT, finished_at TEXT,
    last_error TEXT, created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP, UNIQUE(user_id,source_fingerprint)
);
CREATE TABLE cloud_file_blobs (
    id TEXT PRIMARY KEY, user_id TEXT NOT NULL REFERENCES cloud_users(id) ON DELETE CASCADE,
    file_entity_id TEXT NOT NULL, ledger_id TEXT, attachment_kind TEXT NOT NULL,
    sha256 TEXT NOT NULL, size_bytes INTEGER NOT NULL, mime_type TEXT,
    file_name TEXT NOT NULL, content BLOB NOT NULL, created_by_device_id TEXT,
    created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP, UNIQUE(user_id,file_entity_id)
);
CREATE TABLE beecount_user_profiles (
    user_id TEXT PRIMARY KEY REFERENCES cloud_users(id) ON DELETE CASCADE,
    income_is_red INTEGER, theme_primary_color TEXT, appearance TEXT, ai_config TEXT,
    primary_currency TEXT, avatar_version INTEGER NOT NULL DEFAULT 0,
    avatar_mime_type TEXT, avatar_file_name TEXT, avatar_content BLOB,
    updated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
);
CREATE TABLE beecount_shared_ledgers (
    ledger_id TEXT PRIMARY KEY, storage_user_id TEXT NOT NULL REFERENCES cloud_users(id) ON DELETE CASCADE,
    created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
);
CREATE TABLE beecount_ledger_members (
    ledger_id TEXT NOT NULL REFERENCES beecount_shared_ledgers(ledger_id) ON DELETE CASCADE,
    user_id TEXT NOT NULL REFERENCES cloud_users(id) ON DELETE CASCADE,
    role TEXT NOT NULL, invited_by TEXT REFERENCES cloud_users(id) ON DELETE SET NULL,
    joined_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
    PRIMARY KEY(ledger_id,user_id)
);
CREATE UNIQUE INDEX idx_beecount_ledger_one_owner ON beecount_ledger_members(ledger_id) WHERE role='owner';
CREATE TABLE beecount_ledger_invites (
    code TEXT PRIMARY KEY, ledger_id TEXT NOT NULL REFERENCES beecount_shared_ledgers(ledger_id) ON DELETE CASCADE,
    invited_by TEXT NOT NULL REFERENCES cloud_users(id) ON DELETE CASCADE,
    target_role TEXT NOT NULL DEFAULT 'editor', expires_at TEXT NOT NULL,
    used_at TEXT, used_by TEXT REFERENCES cloud_users(id) ON DELETE SET NULL,
    created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE TABLE photo_staging_items (
    id TEXT PRIMARY KEY, user_id TEXT NOT NULL REFERENCES cloud_users(id) ON DELETE CASCADE,
    source TEXT NOT NULL, client_asset_id TEXT, sha256 TEXT NOT NULL,
    original_name TEXT NOT NULL, media_type TEXT NOT NULL DEFAULT 'image',
    mime_type TEXT NOT NULL, size_bytes INTEGER NOT NULL, captured_at TEXT,
    storage_name TEXT NOT NULL UNIQUE, created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
    expires_at TEXT
);
CREATE UNIQUE INDEX photo_staging_items_client_asset_uq ON photo_staging_items(user_id,source,client_asset_id) WHERE client_asset_id IS NOT NULL;
CREATE TABLE photo_challenge_scores (
    id TEXT PRIMARY KEY, user_id TEXT NOT NULL REFERENCES cloud_users(id) ON DELETE CASCADE,
    staging_id TEXT REFERENCES photo_staging_items(id) ON DELETE SET NULL,
    image_hash TEXT NOT NULL, file_name TEXT, captured_at TEXT, score INTEGER NOT NULL,
    qualified INTEGER NOT NULL, breakdown TEXT NOT NULL, feedback TEXT NOT NULL,
    model TEXT NOT NULL, thumbnail_data_url TEXT, scored_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
    UNIQUE(user_id,image_hash)
);

CREATE TABLE execution_worker_leases (
    lease_name TEXT PRIMARY KEY, owner_id TEXT NOT NULL, lease_until TEXT NOT NULL,
    heartbeat_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
    acquired_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE TABLE file_objects (
    id TEXT PRIMARY KEY, user_id TEXT NOT NULL REFERENCES cloud_users(id) ON DELETE CASCADE,
    domain TEXT NOT NULL, original_name TEXT NOT NULL, mime_type TEXT NOT NULL,
    size_bytes INTEGER NOT NULL, sha256 TEXT NOT NULL, storage_key TEXT NOT NULL UNIQUE,
    entity_type TEXT, entity_id TEXT, status TEXT NOT NULL DEFAULT 'pending',
    upload_attempts INTEGER NOT NULL DEFAULT 0, failure_reason TEXT,
    created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
    available_at TEXT, deleted_at TEXT
);
CREATE UNIQUE INDEX idx_file_objects_owner_domain_hash ON file_objects(user_id,domain,sha256,size_bytes) WHERE deleted_at IS NULL;
CREATE INDEX idx_file_objects_owner_entity ON file_objects(user_id,entity_type,entity_id) WHERE deleted_at IS NULL AND entity_type IS NOT NULL;
