[1.4] - ????-??-??
Changed:
* Made the UI a bit more clean and sleek and also make it mobile friendly.
* Highlight when the king is in check.
* Allow player to keep playing out the game after the puzzle is complete.

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
