CREATE TABLE IF NOT EXISTS publish_settings (
    project_id TEXT PRIMARY KEY,
    version INTEGER NOT NULL CHECK (version = 1),
    settings_json TEXT NOT NULL CHECK (json_valid(settings_json)),
    updated_at TEXT NOT NULL,
    FOREIGN KEY (project_id) REFERENCES projects(id) ON DELETE CASCADE
);

CREATE TABLE IF NOT EXISTS publish_deployments (
    project_id TEXT PRIMARY KEY,
    version INTEGER NOT NULL CHECK (version = 1),
    account_id TEXT NOT NULL,
    worker_name TEXT NOT NULL,
    bucket_name TEXT NOT NULL,
    workers_subdomain TEXT NOT NULL,
    public_url TEXT NOT NULL,
    worker_version_id TEXT,
    corpus_sha256 TEXT NOT NULL,
    last_published_at TEXT NOT NULL,
    cleanup_pending INTEGER NOT NULL DEFAULT 0 CHECK (cleanup_pending IN (0, 1)),
    FOREIGN KEY (project_id) REFERENCES projects(id) ON DELETE CASCADE
);
