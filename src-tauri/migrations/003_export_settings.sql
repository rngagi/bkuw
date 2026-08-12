ALTER TABLE projects
    ADD COLUMN analysis_language TEXT
    CHECK (analysis_language IN ('zh-TW', 'en') OR analysis_language IS NULL);

CREATE TABLE export_settings (
    project_id TEXT PRIMARY KEY,
    version INTEGER NOT NULL CHECK (version = 1),
    settings_json TEXT NOT NULL CHECK (json_valid(settings_json)),
    updated_at TEXT NOT NULL,
    FOREIGN KEY (project_id) REFERENCES projects(id) ON DELETE CASCADE
);
