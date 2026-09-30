-- Manual Mail categories were retired in favor of automatic source grouping.
-- Keep migrations 0005/0006 unchanged for migration-history compatibility, then
-- remove the no-longer-used local category storage here.

DROP TABLE IF EXISTS mail_message_categories;
DROP TABLE IF EXISTS mail_categories;
