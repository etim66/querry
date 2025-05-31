CREATE TABLE IF NOT EXISTS collectionitem(
    id TEXT NOT NULL PRIMARY KEY,
    created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
    name TEXT NOT NULL,
    icon TEXT,
    requests_count INTEGER DEFAULT 0
);

CREATE TABLE IF NOT EXISTS collectionheader(
    id TEXT NOT NULL PRIMARY KEY,
    created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
    name TEXT,
    value TEXT,
    active INTEGER NOT NULL DEFAULT 1, -- Boolean field (0 for false, 1 for true)
    collection_id TEXT NOT NULL REFERENCES collectionitem(id) ON DELETE CASCADE
);

CREATE TABLE IF NOT EXISTS requestitem(
    id TEXT NOT NULL PRIMARY KEY,
    created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
    name TEXT,
    url TEXT,
    protocol TEXT,
    http_method TEXT,
    collection_id TEXT NOT NULL REFERENCES collectionitem(id) ON DELETE CASCADE
);

CREATE TABLE IF NOT EXISTS requestheader(
    id TEXT NOT NULL PRIMARY KEY,
    created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
    key TEXT,
    value TEXT,
    active INTEGER NOT NULL DEFAULT 1, -- Boolean field (0 for false, 1 for true)
    request_id TEXT NOT NULL REFERENCES requestitem(id) ON DELETE CASCADE
);