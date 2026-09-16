-- F10 Profile/Devices requires the Execute Android app to manage its own
-- native auth sessions. New grants receive these scopes from the Rust
-- allow-list; this migration upgrades existing active grants and sessions so
-- deployed clients can obtain them on the next token refresh without forcing a
-- logout/login cycle.

UPDATE auth_app_grants
SET scopes = ARRAY(
        SELECT DISTINCT value
        FROM unnest(scopes || ARRAY['sessions:read', 'sessions:write']::TEXT[]) AS value
        ORDER BY value
    ),
    updated_at = now()
WHERE app_id = 'lifetrace-execute-android'
  AND status = 'active';

UPDATE auth_sessions
SET scopes = ARRAY(
        SELECT DISTINCT value
        FROM unnest(scopes || ARRAY['sessions:read', 'sessions:write']::TEXT[]) AS value
        ORDER BY value
    )
WHERE app_id = 'lifetrace-execute-android'
  AND status = 'active';
