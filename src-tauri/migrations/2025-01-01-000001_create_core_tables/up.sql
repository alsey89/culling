-- Projects table
CREATE TABLE projects (
    id TEXT PRIMARY KEY NOT NULL,
    name TEXT NOT NULL,
    source_path TEXT NOT NULL,
    output_path TEXT NOT NULL,
    exclude_patterns TEXT NOT NULL DEFAULT '[]', -- JSON array
    file_types TEXT NOT NULL DEFAULT '["jpg","jpeg","png","heic","tiff","webp"]', -- JSON array
    scan_status TEXT NOT NULL DEFAULT 'not_started',
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);

-- Assets table
CREATE TABLE assets (
    id TEXT PRIMARY KEY NOT NULL,
    project_id TEXT NOT NULL,
    path TEXT NOT NULL,
    hash TEXT, -- SHA-256 content hash
    perceptual_hash TEXT, -- For similarity detection
    size INTEGER NOT NULL,
    width INTEGER NOT NULL,
    height INTEGER NOT NULL,
    exif_data TEXT, -- JSON blob for EXIF data
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    FOREIGN KEY (project_id) REFERENCES projects (id) ON DELETE CASCADE
);

-- Variant groups table
CREATE TABLE variant_groups (
    id TEXT PRIMARY KEY NOT NULL,
    project_id TEXT NOT NULL,
    group_type TEXT NOT NULL, -- 'exact' or 'similar'
    similarity REAL NOT NULL DEFAULT 0.0, -- 0-100
    suggested_keep TEXT, -- asset_id
    created_at TEXT NOT NULL,
    FOREIGN KEY (project_id) REFERENCES projects (id) ON DELETE CASCADE,
    FOREIGN KEY (suggested_keep) REFERENCES assets (id) ON DELETE SET NULL
);

-- Asset group memberships (many-to-many)
CREATE TABLE asset_groups (
    asset_id TEXT NOT NULL,
    group_id TEXT NOT NULL,
    PRIMARY KEY (asset_id, group_id),
    FOREIGN KEY (asset_id) REFERENCES assets (id) ON DELETE CASCADE,
    FOREIGN KEY (group_id) REFERENCES variant_groups (id) ON DELETE CASCADE
);

-- Decisions table
CREATE TABLE decisions (
    asset_id TEXT PRIMARY KEY NOT NULL,
    state TEXT NOT NULL DEFAULT 'undecided', -- 'keep', 'remove', 'undecided'
    reason TEXT NOT NULL DEFAULT 'manual_no_reason',
    notes TEXT,
    decided_at TEXT NOT NULL,
    FOREIGN KEY (asset_id) REFERENCES assets (id) ON DELETE CASCADE
);

-- Indexes for performance
CREATE INDEX idx_assets_project_id ON assets (project_id);
CREATE INDEX idx_assets_hash ON assets (hash);
CREATE INDEX idx_assets_perceptual_hash ON assets (perceptual_hash);
CREATE INDEX idx_variant_groups_project_id ON variant_groups (project_id);
CREATE INDEX idx_asset_groups_asset_id ON asset_groups (asset_id);
CREATE INDEX idx_asset_groups_group_id ON asset_groups (group_id);
CREATE INDEX idx_decisions_asset_id ON decisions (asset_id);