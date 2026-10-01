-- Optimize the hot Mail read paths used by the inbox list and realtime refresh.

CREATE INDEX IF NOT EXISTS idx_mail_messages_user_received
ON mail_messages(user_id, received_at DESC);

CREATE INDEX IF NOT EXISTS idx_mail_messages_user_account_received
ON mail_messages(user_id, account_id, received_at DESC);

CREATE INDEX IF NOT EXISTS idx_mail_messages_folder_received
ON mail_messages(folder_id, received_at DESC);

CREATE INDEX IF NOT EXISTS idx_mail_folders_user_account_role
ON mail_folders(user_id, account_id, normalized_role);
