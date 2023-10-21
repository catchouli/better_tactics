[1.8.0] - ????-??-??

Added:
* A Puzzle History page, which shows the history of all puzzles reviewed.
* On the 'next puzzle' screen, the puzzle to be reviewed is now saved until it is skipped.
* Skip buttons have been added to the puzzle interface for new puzzles:
  * "Skip this puzzle" when the puzzle is first started, which skips the current puzzle completely
    but doesn't award you any rating points.
  * "Too hard" and "Too easy" buttons, which also skip the puzzle, but correct your rating
    accordingly so you'll see easier or harder puzzles in the future.
  * "Don't repeat this puzzle", which marks the puzzle as complete, but without adding it to spaced
    repetition.
* Automated daily backups are now supported, and can be configured with the BACKUP_ENABLED,
  BACKUP_PATH, and BACKUP_HOUR configuration variables. See CONFIG.md for more details. The backup
  files are functional .sqlite databases that can be loaded back in as the application database,
  but do not include the puzzles, so if they are loaded the puzzles will be reinitialised when the
  application starts.

Removed:
* The configuration variable SQLITE_DB_NAME is now deprecated in favor of DATABASE_URL. If you
  have a SQLITE_DB_NAME set but not a DATABASE_URL, for compatibility DATABASE_URL will be set
  to "sqlite://{SQLITE_DB_NAME}". See CONFIG.md for more details.

Changed:
* Puzzles on the 'next puzzle' page will never be ones you've seen before anymore.
* Changes the .sqlite database to use WAL journaling (write-ahead-logging) so that multiple readers
  can use the database while something is writing to it. Overall the application should be a little
  faster.
* Tweaks the initial puzzle import to be a bit faster, and not lock the database as much. This is
  especially useful in docker, where the initial import was quite slow for me previously, and the
  web app was very slow to load while it was importing. The database import is now a lot faster,
  and page loads are generally instant while it is in progress. API calls that need to write (such
  as reviewing and skipping puzzles) might still need a couple of seconds while puzzles are being
  written to a database but it's much faster overall.

[1.7.0] - 2023-10-19

Changed:
* The puzzle page now loads the next puzzle automatically without refreshing the page.
* Cards leaving learning immediately due to reviewing them as 'Easy' now have their interval set
  to 4 days instead of the previous 1 day. This should allow actually easy puzzles to be seen far
  less frequently.
* The review forecast on the stats screen now allows you to change the number of days shown, and
  review scores allows you to see the data by percentage as well as total value.
* A link to the original lichess puzzle is now included on the puzzle interface.

[1.6.0] - 2023-10-13

Added:
* A review forecast on the stats page which shows the number of reviews due per day for the next
  week.
* A rating history which shows the user's rating growth over time.
* A review score histogram which shows the review scores being picked by the user by puzzle rating.

Changed:
* The user's rating deviation is now hidden from the stats page if it's under 100, the threshold
  under which the user's rating is no longer provisional and should be quite accurate.

[1.5.2] - 2023-10-09

Changed:
* Puzzle themes are now hidden until the puzzle is complete to avoid spoilers.

Fixed:
* Fixes the horizontal scrolling on phones due to the extra header width introduced with the burger
  menu.
* Makes the database import a bit faster, and reduces the spammyness of it.

[1.5.1] - 2023-10-06

Fixed:
* The database wasn't getting created automatically anymore, so the application was failing to
  start if there was no existing database. The database now gets created correctly.

[1.5] - 2023-10-06

Added:
* Added an About page, explaining what's going on and a lot of useful information.
* Added a navigation menu to get to the various pages.
* Added legal move display.

[1.4] - 2023-10-06

Changed:
* Made the UI a bit more clean and sleek and also make it mobile friendly.
* Highlight when the king is in check.
* Allow player to keep playing out the game after the puzzle is complete.
* Added app configuration via environment variables - .env contains all of the available
  configuration variables and can be used to set them.
* The server now starts immediately, and database initialision is done in the background if needed.
  We keep track of the lichess puzzle import using a flag in the database, so if you don't let it
  complete at least once, it will restart every time the application starts until it is completed.
* Add puzzle themes to the puzzle page.

Fixed:
* Fixes a bug where despite the review-ahead time for the day being until 4am, after midnight it'd
  start to show you reviews due until the *following 4am* (i.e. 28 hours later)

[1.3] - 2023-10-01

Changed:
* Tweaked the rating sytstem once more. Once it gets a good idea of your actual rating, and your
  rating deviation goes low enough, it's actually producing extremely good results now, and doesn't
  award too much rating for repeated attempts of puzzles around your level. As a result, I've
  reenabled the ability for every review to update your rating level. I've found that after a while
  your rating stabilizes at an appropriate level and then gradually grows over time which is
  perfect, and that 'Easy' and 'Again' reviews are great at hinting to the system to reevaluate
  your current rating level, because it is too low or too high accordingly.
* 'Reviewing ahead' of the day's reviews (so you can do all of your reviews at one time instead of
  waiting for them to trickle in throughout the day) has been tweaked so that it only applies to
  cards that are out of learning, otherwise they tend to show up again immediately before any cards
  due later the same day. Now, cards in learning don't show up until they're actually due (usually
  at most about 10 minutes after you've last seen them, giving you chance to do some other reviews
  before they come up again.)
* Fixes various bugs with data access and error reporting.

[1.2] - 2023-09-30

Added:
* Support promotion in the UI properly instead of assuming auto-queen.

[1.1] - 2023-09-30

Changed:
* Tweaks the rating system to make the rating grow a bit more appropriately.

Added:
* Adds a debug endpoint for setting the user's rating, e.g. /set_rating/500, in case users need to
  fix their rating without having to modify the database manually.
* Implements premoves in the puzzle UI, and allow the pieces to continue being dragged even while
  the computer is making their move.

[1.0] - 2023-09-29

* Initial prototype version, with working rating system.
