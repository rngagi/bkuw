CREATE TABLE IF NOT EXISTS sense_images (
    id TEXT PRIMARY KEY,
    sense_id TEXT NOT NULL,
    relative_path TEXT NOT NULL UNIQUE,
    original_filename TEXT NOT NULL,
    width INTEGER NOT NULL CHECK (width > 0),
    height INTEGER NOT NULL CHECK (height > 0),
    byte_size INTEGER NOT NULL CHECK (byte_size > 0),
    sha256 TEXT NOT NULL CHECK (length(sha256) = 64),
    sort_order INTEGER NOT NULL DEFAULT 0,
    created_at TEXT NOT NULL,
    FOREIGN KEY (sense_id) REFERENCES senses(id) ON DELETE CASCADE
);

CREATE INDEX IF NOT EXISTS sense_images_sense_order
    ON sense_images(sense_id, sort_order, id);
