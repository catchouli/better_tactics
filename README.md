# better-tactics
<!-- Screenshot gallery -->
<a href="https://raw.githubusercontent.com/catchouli/better_tactics/develop/screenshots/1.png">
  <img src="https://raw.githubusercontent.com/catchouli/better_tactics/develop/screenshots/1.png" width="32%">
</a>
<a href="https://raw.githubusercontent.com/catchouli/better_tactics/develop/screenshots/2.png">
  <img src="https://raw.githubusercontent.com/catchouli/better_tactics/develop/screenshots/2.png" width="32%">
</a>
<a href="https://raw.githubusercontent.com/catchouli/better_tactics/develop/screenshots/3.png">
  <img src="https://raw.githubusercontent.com/catchouli/better_tactics/develop/screenshots/3.png" width="32%">
</a>

A chess tactics trainer that with spaced repetition. New puzzles will be shown to you from the lichess puzzle db, according to your calculated rating level, and then puzzles that need reviewing wil be shown to you daily.

To run it:
* Either grab a build from the releases page and just run it, or download the repo and `cargo +nightly run --release`.
* The lichess puzzles db will be automatically downloaded on the first run and saved to the application's sqlite database.
* Once it says it's serving the site go to http://localhost:3030 to use the app.
* Click the 'New Puzzle' button to see some new puzzles, which will then be added to your review queue, or the 'Review' app to do your daily reviews!

Note: For a standalone/portable build use the release build, as it compiles the static assets in, but the debug build references them from the ./assets directory.

## Warning
This project is pretty functional but still quite experimental. You might encounter issues that require you to reset (or manually repair) your database. In particular, the rating system is still very much WIP.

If you find you need to manually reset your rating or set it to a particular value, you can set it using the debug endpoint `/set_rating/{desired_rating}`, which also resets your rating variance and should allow the app to re-find your rating level at about the given level. (e.g. http://localhost:3030/set_rating/1000)
