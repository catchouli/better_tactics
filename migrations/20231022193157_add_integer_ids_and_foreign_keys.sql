-- Enable foreign keys.
PRAGMA foreign_keys = ON;

-- Create puzzle sources table, and add the initial puzzle source (lichess).
CREATE TABLE puzzle_sources (
    id INTEGER NOT NULL,
    name TEXT NOT NULL,
    PRIMARY KEY (id)
);

INSERT INTO puzzle_sources (id, name) VALUES (0, "lichess");

-- Recreate puzzles table structure to have integer primary keys by first
-- copying the data to a temporary table.
CREATE TEMPORARY TABLE puzzles_temp (
    puzzle_id TEXT NOT NULL,
    fen TEXT NOT NULL,
    moves TEXT NOT NULL,
    rating INTEGER,
    rating_deviation INTEGER,
    popularity INTEGER,
    number_of_plays INTEGER,
    game_url TEXT,
    PRIMARY KEY (puzzle_id)
);

-- Copy over only puzzles that are associated with cards. Copying them all over
-- took way too long, and we want to re-import them anyway to get the themes
-- and openings imported properly.
INSERT OR REPLACE INTO puzzles_temp (puzzle_id, fen, moves, rating, rating_deviation,
    popularity, number_of_plays, game_url)
SELECT puzzles.puzzle_id, fen, moves, rating, rating_deviation, popularity,
    number_of_plays, game_url
FROM puzzles
JOIN cards ON cards.puzzle_id = puzzles.puzzle_id;

INSERT OR REPLACE INTO puzzles_temp (puzzle_id, fen, moves, rating, rating_deviation,
    popularity, number_of_plays, game_url)
SELECT puzzles.puzzle_id, fen, moves, rating, rating_deviation, popularity,
    number_of_plays, game_url
FROM puzzles
JOIN skipped_puzzles ON skipped_puzzles.puzzle_id = puzzles.puzzle_id;

-- Remove the original puzzles table.
DROP TABLE puzzles;

-- Create the final puzzles table, with an integer primary key, puzzle_id
-- renamed to source_id, and themes/opening tags removed.
CREATE TABLE puzzles (
    id INTEGER NOT NULL,
    source INTEGER NOT NULL,
    source_id TEXT NOT NULL,
    fen TEXT NOT NULL,
    moves TEXT NOT NULL,
    rating INTEGER,
    rating_deviation INTEGER,
    popularity INTEGER,
    number_of_plays INTEGER,
    game_url TEXT,
    PRIMARY KEY (id),
    FOREIGN KEY (source) REFERENCES puzzle_sources(id)
);

-- Re-save the puzzles that were associated with cards.
INSERT INTO puzzles (source_id, fen, moves, rating, rating_deviation,
    popularity, number_of_plays, game_url, source)
SELECT puzzle_id, fen, moves, rating, rating_deviation, popularity,
    number_of_plays, game_url, 0
FROM puzzles_temp;

-- Delete the temporary puzzles table.
DROP TABLE puzzles_temp;

-- Create indices for puzzles.
CREATE UNIQUE INDEX puzzle_source_id ON puzzles(source, source_id);
CREATE INDEX puzzle_rating ON puzzles(rating);

-- Create themes and openings tables.
CREATE TABLE themes (
    id INTEGER NOT NULL,
    name TEXT NOT NULL,
    PRIMARY KEY (id)
);

CREATE TABLE openings (
    id INTEGER NOT NULL,
    name TEXT NOT NULL,
    PRIMARY KEY (id)
);

-- Create relationship tables.
CREATE TABLE puzzle_themes (
    puzzle_id INTEGER NOT NULL,
    theme_id INTEGER NOT NULL,
    PRIMARY KEY (puzzle_id, theme_id),
    FOREIGN KEY (puzzle_id) REFERENCES puzzles(id),
    FOREIGN KEY (theme_id) REFERENCES themes(id)
);

CREATE TABLE puzzle_openings (
    puzzle_id INTEGER NOT NULL,
    opening_id INTEGER NOT NULL,
    PRIMARY KEY (puzzle_id, opening_id),
    FOREIGN KEY (puzzle_id) REFERENCES puzzles(id),
    FOREIGN KEY (opening_id) REFERENCES openings(id)
);

-- Force a reimport of all puzzles, themes, and openings.
UPDATE app_data SET lichess_db_imported = 0;

-- Recreate users table to have an integer primary key, and for the existing
-- id to be a username.
ALTER TABLE users RENAME TO users_old;

-- Create new users table.
CREATE TABLE users (
    id INTEGER NOT NULL,
    username TEXT UNIQUE NOT NULL,
    rating INTEGER,
    rating_deviation INTEGER,
    rating_volatility FLOAT,
    next_puzzle INTEGER DEFAULT NULL,
    PRIMARY KEY (id),
    FOREIGN KEY (next_puzzle) REFERENCES puzzles(id)
);

-- Copy over existing user(s).
INSERT INTO users (username, rating, rating_deviation, rating_volatility)
SELECT id AS username, rating, rating_deviation, rating_volatility
FROM users_old;

-- Delete old users table.
DROP TABLE users_old;

-- Recreate reviews table to have an integer primary key, and reference other
-- tables using a foreign key.
ALTER TABLE reviews RENAME TO reviews_old;

-- Create new reviews table.
CREATE TABLE reviews (
    id INTEGER NOT NULL,
    user_id INTEGER NOT NULL,
    puzzle_id INTEGER NOT NULL,
    difficulty INTEGER NOT NULL,
    date TEXT NOT NULL,
    user_rating INTEGER,
    PRIMARY KEY (id),
    FOREIGN KEY (user_id) REFERENCES users(id),
    FOREIGN KEY (puzzle_id) REFERENCES puzzles(id)
);

-- Copy over existing reviews with the new puzzle/user ids.
INSERT INTO reviews (user_id, puzzle_id, difficulty, date, user_rating)
SELECT users.id AS user_id,
       puzzles.id AS puzzle_id,
       reviews_old.difficulty,
       reviews_old.date,
       reviews_old.user_rating
FROM reviews_old
JOIN users ON users.username = reviews_old.user_id
JOIN puzzles ON puzzles.source_id = reviews_old.puzzle_id;

-- Delete the old table.
DROP TABLE reviews_old;

-- Recreate cards table to have an integer primary key, and reference other
-- tables using a foreign key.
ALTER TABLE cards RENAME TO cards_old;

-- Create new cards table.
CREATE TABLE cards (
	id INTEGER NOT NULL,
    user_id INTEGER NOT NULL,
    puzzle_id INTEGER NOT NULL,
    due TEXT NOT NULL,
    interval INTEGER NOT NULL,
    review_count INTEGER NOT NULL,
    ease FLOAT NOT NULL,
    learning_stage INTEGER NOT NULL,
	PRIMARY KEY (id),
    FOREIGN KEY (user_id) REFERENCES users(id),
	FOREIGN KEY (puzzle_id) REFERENCES puzzles(id)
);

CREATE UNIQUE INDEX card_user_puzzle ON cards(user_id, puzzle_id);

-- Copy over existing data.
INSERT INTO cards (user_id, puzzle_id, due, interval, review_count, ease,
    learning_stage)
SELECT users.id AS user_id, puzzles.id AS puzzle_id, due, interval,
    review_count, ease, learning_stage
FROM cards_old
JOIN puzzles ON cards_old.puzzle_id = puzzles.source_id
JOIN users ON users.username = 'local';

-- Delete old table.
DROP TABLE cards_old;

-- Recreate skipped_puzzles table to have an integer primary key, and reference
-- other tables using a foreign key.
ALTER TABLE skipped_puzzles RENAME TO skipped_puzzles_old;

-- Create new skipped_puzzles table.
CREATE TABLE skipped_puzzles (
	id INTEGER NOT NULL,
    user_id INTEGER NOT NULL,
    puzzle_id INTEGER NOT NULL,
    date TEXT NOT NULL,
	PRIMARY KEY (id),
	FOREIGN KEY (user_id) REFERENCES users(id),
	FOREIGN KEY (puzzle_id) REFERENCES puzzles(id)
);

-- Copy over existing data.
INSERT INTO skipped_puzzles (user_id, puzzle_id, date)
SELECT users.id, puzzles.id, skipped_puzzles_old.date
FROM skipped_puzzles_old
JOIN users ON skipped_puzzles_old.user_id = users.username
JOIN puzzles ON skipped_puzzles_old.puzzle_id = puzzles.source_id;

-- Delete old table.
DROP TABLE skipped_puzzles_old;
