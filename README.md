# better-tactics
A command line app that pulls tactics puzzles from lichess's database by rating range and exports them to PGN format for training.

To run it:
* Get lichess_db_puzzle.csv.zst from `https://database.lichess.org/#puzzles`
* Run `cargo run` to run it and generate a pgn file
* Edit MIN_RATING, MAX_RATING, and MAX_PUZZLES at the top of main.rs until I add some kind of options
