CREATE TABLE runs(
    id uuid NOT NULL PRIMARY KEY,
    run_time timestamptz NOT NULL,
    name TEXT NOT NULL
);
