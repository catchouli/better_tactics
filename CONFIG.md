# Configuration
The application can be configured by setting the values in <a href="https://github.com/catchouli/better_tactics/blob/main/src/app.rs#L17">AppConfig</a>. This can be done either using environment variables, or a .env file. See <a href="https://github.com/catchouli/better_tactics/blob/main/.env">the development .env file</a> for an example. The application config is printed out on startup, and any errors parsing config values are logged as errors, so it's easy to check if your configuration is being picked up.

## Supported configuration values

# General
| Environment Variable | Default | Description |
| --- | --- | --- |
| BIND_INTERFACE | 127.0.0.1 | The IP address to bind the webserver to. By default, it binds to 127.0.0.1, so can only be accessed locally. To make it available externally, set BIND_INTERFACE to 0.0.0.0 to bind all interfaces. |
| BIND_PORT | 3030 | The port to bind the web server to, the default is 3030 |
| DATABASE_URL | sqlite://puzzles.sqlite | The url of the database. "sqlite://path/to/puzzles.sqlite" refers to the relative path "./path/to/puzzles.sqlite", while "sqlite:///path/to/puzzles.sqlite" with an additional slash at the start of the path refers to the absolute path /path/to/puzzles.sqlite |

# Backups
Once enabled, backups will run once a day. If a backup is due to be run today and hasn't already, it will be attempted when the application starts. Automated backups are stored as .sqlite files with only the user data and no puzzles, but can be loaded back in as the main application database, at which point the puzzles will be automatically re-initialised from the lichess database.

| Environment Variable | Default | Description |
| --- | --- | --- |
| BACKUP_ENABLED | false | Whether or not daily automated backups are enabled |
| BACKUP_PATH | ./backups | The path to store automated backups |
| BACKUP_HOUR | 4 | The hour (local time) at which the automated backup is scheduled to run, i.e. 4 is 4am |

# Spaced repetition
See <a href="https://super-memory.com/english/ol/sm2.htm">Supermemo 2 algorithm</a> for information about the spaced repetition algorithm.

| Environment Variable | Default | Description |
| --- | --- | --- |
| SRS_DEFAULT_EASE | 2.5 | The default ease factor, the multiplier for card interval for each review, which goes up for 'easy' reviews and down for 'hard' or 'again' reviews |
| SRS_MINIMUM_EASE | 1.3 | The minimum ease factor when the ease is decreased |
| SRS_EASY_BONUS | 1.3 | The extra multiplier to the card interval when a card is review as 'easy' |
| SRS_DAY_END_HOUR | 4 | The hour (local time) at which the day is considered to start/end. The review queue will automatically include cards up to this time, so user can review all of today's cards at once. |
| SRS_REVIEW_ORDER | DueTime | The order for puzzles to show up when reviewing. Valid values are: DueTime (the time the card is due), PuzzleRating (lower rated puzzles are shown first), and Random (reviews are shown in a random order from the pool of due reviews).|

## Deprecated configuration values
| Environment Variable | Description |
| ---  | --- |
| SQLITE_DB_NAME | Setting DATABASE_URL in the format "sqlite:///path/to/filename.sqlite" should be preferred. If DATABASE_URL is not set but SQLITE_DB_NAME is, DATABASE_URL will be initialised as "sqlite:///" + SQLITE_DB_NAME for compatibility. |
