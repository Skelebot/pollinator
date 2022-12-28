CREATE TABLE polls (
    id INTEGER PRIMARY KEY,
    randpart TEXT NOT NULL,
    type TEXT NOT NULL,
    name TEXT NOT NULL,
    date_created TEXT NOT NULL,
    admin_link TEXT NOT NULL,
    voters INTEGER NOT NULL,
    format_data BLOB NOT NULL
);
