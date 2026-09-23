-- Mail workspace compatibility for the SQLite single-container runtime.

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

INSERT INTO mail_identities (
    id,user_id,account_id,email_address,display_name,is_default
)
SELECT id,user_id,id,email_address,display_name,1
FROM mail_accounts
WHERE deleted_at IS NULL
ON CONFLICT DO NOTHING;

ALTER TABLE mail_drafts
ADD COLUMN identity_id TEXT REFERENCES mail_identities(id) ON DELETE SET NULL;

UPDATE mail_drafts
SET identity_id = (
    SELECT i.id
    FROM mail_identities i
    WHERE i.account_id=mail_drafts.account_id
      AND i.is_default=1
      AND i.deleted_at IS NULL
    LIMIT 1
)
WHERE identity_id IS NULL;

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
