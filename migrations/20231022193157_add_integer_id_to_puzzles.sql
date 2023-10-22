CREATE TEMPORARY TABLE puzzles_temp (
    puzzle_id TEXT NOT NULL,
    fen TEXT NOT NULL,
    moves TEXT NOT NULL,
    rating INTEGER,
	rating_deviation INTEGER,
	popularity INTEGER,
	number_of_plays INTEGER,
	game_url TEXT
);

INSERT INTO puzzles_temp (puzzle_id, fen, moves, rating, rating_deviation, popularity, number_of_plays, game_url)
SELECT puzzle_id, fen, moves, rating, rating_deviation, popularity, number_of_plays, game_url FROM puzzles;

DROP TABLE puzzles;

CREATE TABLE puzzles (
	id INTEGER,
    source_id TEXT NOT NULL,
    fen TEXT NOT NULL,
    moves TEXT NOT NULL,
    rating INTEGER,
	rating_deviation INTEGER,
	popularity INTEGER,
	number_of_plays INTEGER,
	game_url TEXT,
	PRIMARY KEY (id)
);

INSERT INTO puzzles (source_id, fen, moves, rating, rating_deviation, popularity, number_of_plays, game_url)
SELECT puzzle_id, fen, moves, rating, rating_deviation, popularity, number_of_plays, game_url FROM puzzles_temp;

DROP TABLE puzzles_temp;

CREATE INDEX puzzle_source_id ON puzzles(source_id);
CREATE INDEX puzzle_rating ON puzzles(rating);
