-- Local user-defined categories for Mail.
--
-- Assignments bind to (account_id, content_hash) rather than the transient
-- mail_messages row id so categories survive IMAP MOVE/reconciliation.

CREATE TABLE mail_categories (
    id TEXT PRIMARY KEY,
    user_id TEXT NOT NULL REFERENCES cloud_users(id) ON DELETE CASCADE,
    name TEXT NOT NULL,
    created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE UNIQUE INDEX idx_mail_categories_user_name
ON mail_categories(user_id, lower(name));

CREATE TABLE mail_message_categories (
    user_id TEXT NOT NULL REFERENCES cloud_users(id) ON DELETE CASCADE,
    account_id TEXT NOT NULL REFERENCES mail_accounts(id) ON DELETE CASCADE,
    content_hash TEXT NOT NULL,
    category_id TEXT NOT NULL REFERENCES mail_categories(id) ON DELETE CASCADE,
    created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
    PRIMARY KEY(user_id, account_id, content_hash, category_id)
);

CREATE INDEX idx_mail_message_categories_category
ON mail_message_categories(user_id, category_id);

CREATE INDEX idx_mail_message_categories_message
ON mail_message_categories(user_id, account_id, content_hash);
