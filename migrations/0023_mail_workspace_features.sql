-- LifeTrace Mail workspace product layer: identities and first-class draft identity mapping.

CREATE TABLE mail_identities (
    id UUID PRIMARY KEY,
    user_id UUID NOT NULL REFERENCES cloud_users(id) ON DELETE CASCADE,
    account_id UUID NOT NULL REFERENCES mail_accounts(id) ON DELETE CASCADE,
    email_address TEXT NOT NULL,
    display_name TEXT,
    reply_to TEXT,
    signature_html TEXT,
    is_default BOOLEAN NOT NULL DEFAULT FALSE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    deleted_at TIMESTAMPTZ
);

CREATE UNIQUE INDEX idx_mail_identities_account_address
ON mail_identities(account_id, lower(email_address))
WHERE deleted_at IS NULL;

CREATE UNIQUE INDEX idx_mail_identities_account_default
ON mail_identities(account_id)
WHERE is_default=TRUE AND deleted_at IS NULL;

INSERT INTO mail_identities (
    id,user_id,account_id,email_address,display_name,is_default
)
SELECT gen_random_uuid(),user_id,id,email_address,display_name,TRUE
FROM mail_accounts
WHERE deleted_at IS NULL
ON CONFLICT DO NOTHING;

ALTER TABLE mail_drafts
ADD COLUMN identity_id UUID REFERENCES mail_identities(id) ON DELETE SET NULL;

UPDATE mail_drafts d
SET identity_id = i.id
FROM mail_identities i
WHERE d.account_id=i.account_id
  AND i.is_default=TRUE
  AND i.deleted_at IS NULL
  AND d.identity_id IS NULL;

CREATE INDEX idx_mail_drafts_user_updated
ON mail_drafts(user_id, updated_at DESC)
WHERE state='draft';
