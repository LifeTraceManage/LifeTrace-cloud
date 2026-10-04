-- Travel sync introduces a new product scope after Desktop/Web grants and
-- sessions may already exist. Append the new scopes without replacing any
-- user-customized existing scopes.

UPDATE auth_app_grants
SET scopes = CASE
      WHEN EXISTS (SELECT 1 FROM json_each(auth_app_grants.scopes) WHERE value='travel:read')
        THEN scopes
      ELSE json_insert(scopes, '$[#]', 'travel:read')
    END,
    updated_at = CURRENT_TIMESTAMP
WHERE app_id IN ('lifetrace-desktop', 'lifetrace-web')
  AND status='active'
  AND json_valid(scopes);

UPDATE auth_app_grants
SET scopes = CASE
      WHEN EXISTS (SELECT 1 FROM json_each(auth_app_grants.scopes) WHERE value='travel:write')
        THEN scopes
      ELSE json_insert(scopes, '$[#]', 'travel:write')
    END,
    updated_at = CURRENT_TIMESTAMP
WHERE app_id IN ('lifetrace-desktop', 'lifetrace-web')
  AND status='active'
  AND json_valid(scopes);

UPDATE auth_sessions
SET scopes = CASE
      WHEN EXISTS (SELECT 1 FROM json_each(auth_sessions.scopes) WHERE value='travel:read')
        THEN scopes
      ELSE json_insert(scopes, '$[#]', 'travel:read')
    END
WHERE app_id IN ('lifetrace-desktop', 'lifetrace-web')
  AND status='active'
  AND json_valid(scopes);

UPDATE auth_sessions
SET scopes = CASE
      WHEN EXISTS (SELECT 1 FROM json_each(auth_sessions.scopes) WHERE value='travel:write')
        THEN scopes
      ELSE json_insert(scopes, '$[#]', 'travel:write')
    END
WHERE app_id IN ('lifetrace-desktop', 'lifetrace-web')
  AND status='active'
  AND json_valid(scopes);
