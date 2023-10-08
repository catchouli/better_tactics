-- Add integer primary keys to puzzle table. To do this, we have to create a new table, copy over
-- the data, and then remove the old table.
CREATE TABLE puzzles_new (
    id INTEGER NOT NULL,
    source_id TEXT NOT NULL,
    fen TEXT NOT NULL,
    moves TEXT NOT NULL,
    rating INTEGER NOT NULL,
    rating_deviation INTEGER NOT NULL,
    popularity INTEGER NOT NULL,
    number_of_plays INTEGER NOT NULL,
    game_url TEXT NOT NULL,
    PRIMARY KEY(id)
);

INSERT INTO puzzles_new (source_id, fen, moves, rating, rating_deviation, popularity, number_of_plays, game_url)
	SELECT puzzle_id AS source_id, fen, moves, rating, rating_deviation, popularity, number_of_plays, game_url
        FROM puzzles;

-- Replace puzzles table with new puzzles table.
DROP TABLE puzzles;
ALTER TABLE puzzles_new RENAME TO puzzles;
CREATE INDEX puzzles_source_id ON puzzles(source_id);
CREATE INDEX puzzles_rating ON puzzles(rating);
