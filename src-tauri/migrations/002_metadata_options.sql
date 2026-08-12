CREATE TABLE metadata_options (
    id TEXT PRIMARY KEY,
    project_id TEXT NOT NULL,
    kind TEXT NOT NULL CHECK (kind IN ('part_of_speech', 'semantic_domain')),
    value TEXT NOT NULL CHECK (trim(value) <> ''),
    sort_order INTEGER NOT NULL DEFAULT 0,
    UNIQUE (project_id, kind, value),
    FOREIGN KEY (project_id) REFERENCES projects(id) ON DELETE CASCADE
);

CREATE INDEX metadata_options_project_kind_order
    ON metadata_options(project_id, kind, sort_order, value);
