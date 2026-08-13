ALTER TABLE lexical_entries ADD COLUMN section_override TEXT;

CREATE TABLE entry_sort_settings (
    project_id TEXT PRIMARY KEY,
    version INTEGER NOT NULL CHECK (version = 1),
    settings_json TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    FOREIGN KEY (project_id) REFERENCES projects(id) ON DELETE CASCADE
);

CREATE TABLE manual_sort_layouts (
    project_id TEXT PRIMARY KEY,
    version INTEGER NOT NULL CHECK (version = 1),
    layout_json TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    FOREIGN KEY (project_id) REFERENCES projects(id) ON DELETE CASCADE
);
