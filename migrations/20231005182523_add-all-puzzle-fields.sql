-- Add extra puzzle fields to puzzle database, with sensible default values
ALTER TABLE puzzles ADD COLUMN rating_deviation INTEGER DEFAULT 50;
ALTER TABLE puzzles ADD COLUMN popularity INTEGER DEFAULT 0;
ALTER TABLE puzzles ADD COLUMN number_of_plays INTEGER DEFAULT 0;
ALTER TABLE puzzles ADD COLUMN themes TEXT DEFAULT '';
ALTER TABLE puzzles ADD COLUMN game_url TEXT DEFAULT '';
ALTER TABLE puzzles ADD COLUMN opening_tags TEXT DEFAULT '';

-- Force a reimport of the lichess database to replace those default values
UPDATE app_data SET lichess_db_imported = 0;

-- Remove unnecessary db_version field while we're here
ALTER TABLE app_data DROP COLUMN db_version;
