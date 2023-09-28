# better-tactics
A chess tactics trainer that with spaced repetition. New puzzles will be shown to you according to your current rating range (using the lichess puzzle db), and then puzzles that need reviewing wil be shown to you daily.

To run it:
* Just `cargo run`, the lichess puzzles db will be automatically downloaded on the first run and saved to the application's sqlite database
* Once it says it's serving the site go to http://localhost:3030

![preview](https://raw.githubusercontent.com/catchouli/better_tactics/main/preview.png)
