CREATE TABLE hashes(
    id uuid NOT NULL,
    PRIMARY KEY (id),
    hash BYTEA NOT NULL,
    hashing_method_name TEXT NOT NULL,
    modified_image uuid 
        REFERENCES modified_images (id)
);

