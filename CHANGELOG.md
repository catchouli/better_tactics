[1.3] - ????-??-??

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
