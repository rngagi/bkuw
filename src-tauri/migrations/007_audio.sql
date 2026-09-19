CREATE TABLE audio_attachments (
    id TEXT PRIMARY KEY,
    sense_id TEXT REFERENCES senses(id) ON DELETE CASCADE,
    example_id TEXT REFERENCES examples(id) ON DELETE CASCADE,
    relative_path TEXT NOT NULL UNIQUE,
    original_filename TEXT NOT NULL,
    duration_ms INTEGER NOT NULL CHECK(duration_ms > 0),
    byte_size INTEGER NOT NULL CHECK(byte_size > 0),
    sha256 TEXT NOT NULL,
    sort_order INTEGER NOT NULL,
    created_at TEXT NOT NULL,
    CHECK ((sense_id IS NOT NULL) + (example_id IS NOT NULL) = 1)
);
CREATE INDEX audio_sense ON audio_attachments(sense_id, sort_order);
CREATE INDEX audio_example ON audio_attachments(example_id, sort_order);
