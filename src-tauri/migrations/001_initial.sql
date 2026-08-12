CREATE TABLE projects (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL CHECK (trim(name) <> ''),
    language_name TEXT,
    language_code TEXT,
    description TEXT,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);

CREATE TABLE writing_systems (
    id TEXT PRIMARY KEY,
    project_id TEXT NOT NULL,
    name TEXT NOT NULL CHECK (trim(name) <> ''),
    type TEXT NOT NULL CHECK (type IN (
        'orthography', 'romanization', 'transliteration',
        'phonemic', 'phonetic', 'other'
    )),
    script_code TEXT,
    language_tag TEXT,
    display_role TEXT CHECK (display_role IN ('primary', 'secondary')),
    sort_order INTEGER NOT NULL DEFAULT 0,
    font_family TEXT,
    notes TEXT,
    FOREIGN KEY (project_id) REFERENCES projects(id) ON DELETE CASCADE
);

CREATE UNIQUE INDEX writing_systems_one_primary
    ON writing_systems(project_id) WHERE display_role = 'primary';
CREATE UNIQUE INDEX writing_systems_one_secondary
    ON writing_systems(project_id) WHERE display_role = 'secondary';
CREATE INDEX writing_systems_project_order
    ON writing_systems(project_id, sort_order, name);

CREATE TABLE lexical_entries (
    id TEXT PRIMARY KEY,
    project_id TEXT NOT NULL,
    notes TEXT,
    revision INTEGER NOT NULL DEFAULT 0,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    deleted_at TEXT,
    FOREIGN KEY (project_id) REFERENCES projects(id) ON DELETE CASCADE
);

CREATE INDEX lexical_entries_project_live
    ON lexical_entries(project_id, deleted_at, created_at);

CREATE TABLE entry_forms (
    id TEXT PRIMARY KEY,
    entry_id TEXT NOT NULL,
    writing_system_id TEXT NOT NULL,
    text TEXT NOT NULL,
    search_key TEXT NOT NULL,
    variant_label TEXT,
    dialect TEXT,
    status TEXT,
    notes TEXT,
    sort_order INTEGER NOT NULL DEFAULT 0,
    FOREIGN KEY (entry_id) REFERENCES lexical_entries(id) ON DELETE CASCADE,
    FOREIGN KEY (writing_system_id) REFERENCES writing_systems(id) ON DELETE RESTRICT
);

CREATE INDEX entry_forms_entry_order
    ON entry_forms(entry_id, sort_order);
CREATE INDEX entry_forms_writing_system
    ON entry_forms(writing_system_id);

CREATE TABLE senses (
    id TEXT PRIMARY KEY,
    entry_id TEXT NOT NULL,
    gloss TEXT,
    definition TEXT,
    part_of_speech TEXT,
    semantic_domain TEXT,
    sort_order INTEGER NOT NULL DEFAULT 0,
    FOREIGN KEY (entry_id) REFERENCES lexical_entries(id) ON DELETE CASCADE
);

CREATE INDEX senses_entry_order ON senses(entry_id, sort_order);

CREATE TABLE examples (
    id TEXT PRIMARY KEY,
    sense_id TEXT NOT NULL,
    translation TEXT,
    notes TEXT,
    sort_order INTEGER NOT NULL DEFAULT 0,
    FOREIGN KEY (sense_id) REFERENCES senses(id) ON DELETE CASCADE
);

CREATE INDEX examples_sense_order ON examples(sense_id, sort_order);

CREATE TABLE example_forms (
    id TEXT PRIMARY KEY,
    example_id TEXT NOT NULL,
    writing_system_id TEXT NOT NULL,
    text TEXT NOT NULL,
    sort_order INTEGER NOT NULL DEFAULT 0,
    FOREIGN KEY (example_id) REFERENCES examples(id) ON DELETE CASCADE,
    FOREIGN KEY (writing_system_id) REFERENCES writing_systems(id) ON DELETE RESTRICT
);

CREATE INDEX example_forms_example_order
    ON example_forms(example_id, sort_order);
CREATE INDEX example_forms_writing_system
    ON example_forms(writing_system_id);

CREATE TABLE entry_relations (
    id TEXT PRIMARY KEY,
    source_entry_id TEXT NOT NULL,
    target_entry_id TEXT,
    relation_type TEXT NOT NULL CHECK (relation_type IN ('root', 'base')),
    fallback_text TEXT,
    notes TEXT,
    sort_order INTEGER NOT NULL DEFAULT 0,
    CHECK (target_entry_id IS NOT NULL OR trim(COALESCE(fallback_text, '')) <> ''),
    CHECK (target_entry_id IS NULL OR source_entry_id <> target_entry_id),
    FOREIGN KEY (source_entry_id) REFERENCES lexical_entries(id) ON DELETE CASCADE,
    FOREIGN KEY (target_entry_id) REFERENCES lexical_entries(id) ON DELETE SET NULL
);

CREATE INDEX entry_relations_source
    ON entry_relations(source_entry_id, sort_order);
CREATE INDEX entry_relations_target ON entry_relations(target_entry_id);
