-- Assets attachments reuse the generic EPIC-12 file service. The dedicated
-- Assets application receives generic file scopes but is restricted in the
-- route layer to the assets_attachments domain.

ALTER TABLE file_objects
DROP CONSTRAINT IF EXISTS file_objects_domain_check;

ALTER TABLE file_objects
ADD CONSTRAINT file_objects_domain_check
CHECK (domain IN (
    'finance_imports',
    'notes_attachments',
    'english_audio',
    'photos',
    'workout_imports',
    'backups',
    'assets_attachments'
));

-- New grants receive these scopes from the Rust allow-list. Upgrade existing
-- grants/sessions so a deployed Assets client can use the file service without
-- forcing an account reset.
UPDATE auth_app_grants
SET scopes = ARRAY(
        SELECT DISTINCT value
        FROM unnest(scopes || ARRAY['files:read', 'files:write']::TEXT[]) AS value
        ORDER BY value
    ),
    updated_at = now()
WHERE app_id = 'lifetrace-assets'
  AND status = 'active';

UPDATE auth_sessions
SET scopes = ARRAY(
        SELECT DISTINCT value
        FROM unnest(scopes || ARRAY['files:read', 'files:write']::TEXT[]) AS value
        ORDER BY value
    )
WHERE app_id = 'lifetrace-assets'
  AND status = 'active';
