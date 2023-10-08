# better-tactics

Better Tactics is a Chess tactics trainer that uses the concept of
<a href="https://en.wikipedia.org/wiki/Spaced_repetition">Spaced Repetition</a>
to help you master chess tactics. The idea is to help you gain calculation experience
and tactical pattern recognition by repeating puzzles over time. Puzzles you've seen
will get queued up for review daily, and Puzzles you find hard will be repeated more
frequently, while puzzles you find easy will be repeated far less often to make the
process more efficient.

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

To run it:
* Grab a build from the releases page and just run it, or download the repo and run `cargo +nightly run --release`\*
* Once it says it's serving the site go to http://localhost:3030 to use the app.
* The lichess puzzles db will be automatically downloaded in the background and saved to the application's sqlite database.
* Click the 'New Puzzle' button to see some new puzzles, which will then be added to your review queue, or the 'Review' app to do your daily reviews!

\* Note: For a standalone/portable build use the release build, as it compiles the static assets in, but the debug build references them from the ./assets directory.


# How to use

To begin, start by practicing some new tactics puzzles on the Practice page. There, a new puzzle of an appropriate difficulty will be picked, and you'll be given the chance to study the puzzle and then identify and input the forcing line which results in checkmate, wins you material, or gains you some other advantage. After you've played some puzzles, you'll have daily reviews, which you can do on the Review page.

When you complete a puzzle, you'll be given the option to score it by how difficult you found it. The Spaced Repetition algorithm will then queue up the puzzle to be shown again to you in the future, depending on the score you picked.

If you get really stuck on a puzzle, you can click the 'Analyse' link next to the puzzle to be be taken to the Analysis Board on <a href="https://lichess.org">lichess.org</a>, where you can analyse the puzzle using an engine to try and figure out what you're
missing.

# How it works
When you complete a puzzle, the difficulty score you select will be used to calculate how long it should be until you see the puzzle again, and also to update your rating. The scores are interpreted as follows:

- 'Again' means that you failed to solve the puzzle on the first and need to review it again in the near future, in which case the Spaced Repetition algorithm will set the puzzle to be 're-learned', and you'll see it again the same day.
- 'Hard' meaning that you found the solution but that it was quite challenging, in which case you'll see the puzzle again in about the same amount of time since you last saw it.
- 'Good' is the neutral answer for a successful review, and the one that should be used primarily. After picking it the puzzle's review interval will be increased, and you'll see it less and less frequently over time.
- 'Easy' indicates you didn't find the puzzle very challenging at all, and is a good hint to the algorithm that you don't need to see it again very soon at all, and will cause its review interval to increase significantly.

The review button for each difficulty shows you the amount of time until you'll see that puzzle again if you pick that difficulty.

The difficulty you select is also used to calculate you a rating, according to the difficulty level of the puzzle, and how difficult you found it. 'Good' reviews will cause your rating to grow slowly over time, while 'Again' or 'Easy' reviews may cause larger swings in your rating. Initially, the algorithm will be very uncertain about your rating, and you may experience large swings, but this allows it to quickly find the right rating level for you as it becomes more and more accurate with each puzzle you complete. The rating algorithm used is <a href="https://en.wikipedia.org/wiki/Glicko_rating_system#Glicko-2_algorithm">Glicko2</a>, a common rating system for online chess and competitive games.

# Acknowledgements

Made using <a href="https://www.rust-lang.org/">Rust</a>, <a href="https://github.com/seanmonstar/warp">warp</a>, and <a href="https://github.com/djc/askama">askama</a>. The Spaced Repetition algorithm used is the <a href="https://super-memory.com/english/ol/sm2.htm">SuperMemo 2 Algorithm</a>.

The puzzles are sourced from the <a href="https://database.lichess.org/#puzzles">lichess puzzles database</a>, which is amazing. Thanks, Thibault and the lichess community!

The chess board is also lichess's open source <a href="https://github.com/lichess-org/chessground">chessground</a> chess board
component, and the legal move detection and uses <a href="https://github.com/jhlywa/chess.js">chess.js</a>.



## Warning
This project is pretty functional but still quite experimental. You might encounter issues that require you to reset (or manually repair) your database.

The ratings are currently a bit of a WIP but if you find you need to manually reset your rating or set it to a particular value, you can set it using the debug endpoint `/set_rating/{desired_rating}`, which also resets your rating variance and should allow the app to re-find your rating level at about the given level. (e.g. http://localhost:3030/set_rating/1000)
