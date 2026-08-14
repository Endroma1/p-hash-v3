BEGIN;
    CREATE TABLE runs(
        id uuid NOT NULL PRIMARY KEY,
        run_time timestamptz NOT NULL,
        name TEXT NOT NULL
    );
    ALTER TABLE images
        ADD COLUMN run_id uuid REFERENCES runs(id);

    DO $$
    DECLARE
        new_run_id uuid := uuidv4();
        time timestamptz := NOW();

    BEGIN
        INSERT INTO runs(id, run_time, name) VALUES (new_run_id, time, 'default_run');

        UPDATE images SET run_id = new_run_id WHERE run_id IS NULL;

    END $$;
        ALTER TABLE images ALTER COLUMN run_id SET NOT NULL;
COMMIT;


