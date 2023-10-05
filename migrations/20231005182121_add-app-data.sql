CREATE TABLE IF NOT EXISTS app_data (
    environment TEXT PRIMARY KEY,
    db_version INTEGER NOT NULL,
    lichess_db_imported BOOLEAN NOT NULL
);
INSERT OR IGNORE INTO app_data (environment, db_version, lichess_db_imported)
    VALUES ('', 0, 0);
