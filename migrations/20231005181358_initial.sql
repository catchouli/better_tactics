CREATE TABLE IF NOT EXISTS puzzles (
    puzzle_id TEXT PRIMARY KEY,
    fen TEXT NOT NULL,
    moves TEXT NOT NULL,
    rating INTEGER NOT NULL
);
CREATE TABLE IF NOT EXISTS cards (
    puzzle_id TEXT PRIMARY KEY,
    due TEXT NOT NULL,
    interval INTEGER NOT NULL,
    review_count INTEGER NOT NULL,
    ease FLOAT NOT NULL,
    learning_stage INTEGER NOT NULL
);
DROP TABLE IF EXISTS users;
CREATE TABLE IF NOT EXISTS users_v2 (
    id TEXT PRIMARY KEY,
    rating INTEGER NOT NULL,
    rating_deviation INTEGER NOT NULL,
    rating_volatility FLOAT NOT NULL
);
CREATE TABLE IF NOT EXISTS reviews (
    user_id TEXT NOT NULL,
    puzzle_id TEXT NOT NULL,
    difficulty INTEGER NOT NULL,
    date TEXT NOT NULL
);
INSERT OR IGNORE INTO users_v2 (id, rating, rating_deviation, rating_volatility)
    VALUES ('local', 500, 250, 0.06);
CREATE INDEX IF NOT EXISTS user_id ON users_v2(id);
CREATE INDEX IF NOT EXISTS card_id ON cards(puzzle_id);
CREATE INDEX IF NOT EXISTS puzzle_id ON puzzles(puzzle_id);
CREATE INDEX IF NOT EXISTS puzzle_rating ON puzzles(rating);
