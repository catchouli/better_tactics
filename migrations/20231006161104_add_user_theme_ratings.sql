CREATE TABLE user_theme_ratings (
	user_id TEXT NOT NULL,
	theme TEXT NOT NULL,
	rating INTEGER NOT NULL,
	rating_deviation INTEGER NOT NULL,
	PRIMARY KEY (user_id, theme),
    FOREIGN KEY (user_id) REFERENCES users(id)
);

CREATE TABLE puzzle_themes (
    puzzle_id TEXT NOT NULL,
    theme TEXT NOT NULL,
    PRIMARY KEY (puzzle_id, theme),
    FOREIGN KEY (puzzle_id) REFERENCES puzzles(puzzle_id)
);

CREATE TABLE puzzle_opening_tags (
    puzzle_id TEXT NOT NULL,
    opening TEXT NOT NULL,
    PRIMARY KEY (puzzle_id, opening),
    FOREIGN KEY (puzzle_id) REFERENCES puzzles(puzzle_id)
);

-- Remove the pointless puzzle themes and opening tags columns on the puzzle db.
ALTER TABLE puzzles DROP COLUMN themes;
ALTER TABLE puzzles DROP COLUMN opening_tags;

-- Force a reimport of the lichess database to populate the puzzle themes and opening tags
UPDATE app_data SET lichess_db_imported = 0;
