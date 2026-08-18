-- Command snippets: saved terminal commands, optionally scoped to a host
-- (by `host_id`) or to all hosts in a host group (by `host_group`).
-- Rows with both scope columns NULL are global snippets.
CREATE TABLE IF NOT EXISTS snippet (
    "id"          TEXT PRIMARY KEY,
    "name"        TEXT NOT NULL,
    "command"     TEXT NOT NULL,
    "description" TEXT,
    "group"       TEXT,
    "tags"        TEXT NOT NULL DEFAULT '[]',
    "host_id"     TEXT REFERENCES host("id") ON DELETE SET NULL,
    "host_group"  TEXT,
    "created_at"  TEXT NOT NULL DEFAULT (datetime('now')),
    "updated_at"  TEXT NOT NULL DEFAULT (datetime('now'))
);
CREATE INDEX IF NOT EXISTS snippet_group_idx ON snippet("group");
CREATE INDEX IF NOT EXISTS snippet_name_idx ON snippet("name");
CREATE INDEX IF NOT EXISTS snippet_host_id_idx ON snippet("host_id");
CREATE INDEX IF NOT EXISTS snippet_host_group_idx ON snippet("host_group");
