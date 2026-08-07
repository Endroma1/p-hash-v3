CREATE TABLE modified_images(
    id uuid NOT NULL,
    PRIMARY KEY (id),
    modification TEXT NOT NULL,
    image_id uuid NOT NULL
        REFERENCES images (id)
);
