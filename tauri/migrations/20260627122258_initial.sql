-- Host
CREATE TABLE IF NOT EXISTS host (
    "id" TEXT PRIMARY KEY,
    "name" TEXT NOT NULL,
    "host" TEXT NOT NULL,
    "port" INTEGER NOT NULL DEFAULT 22,
    "username" TEXT NOT NULL,
    "protocol" TEXT NOT NULL DEFAULT 'ssh',
    "group" TEXT,
    "auth_method" TEXT NOT NULL DEFAULT 'password',
    "key_name" TEXT,
    "tags" TEXT NOT NULL DEFAULT '[]',
    "created_at" TEXT NOT NULL DEFAULT (datetime('now')),
    "updated_at" TEXT NOT NULL DEFAULT (datetime('now'))
);
CREATE INDEX IF NOT EXISTS host_group_idx ON host("group");
CREATE INDEX IF NOT EXISTS host_name_idx ON host("name");

-- Known host
CREATE TABLE IF NOT EXISTS known_host (
    "host" TEXT NOT NULL,
    "port" INTEGER NOT NULL,
    "key_type" TEXT NOT NULL,
    "fingerprint" TEXT NOT NULL,
    "created_at" TEXT NOT NULL DEFAULT (datetime('now')),
    -- Constraints
    PRIMARY KEY ("host", "port")
);
CREATE INDEX IF NOT EXISTS known_host_host_port_idx ON known_host("host", "port");

-- Encrypted credential storage (AES-256-GCM).
-- Single-key design: `service` uniquely identifies the credential.
--   Host passwords:  `sheil.host.<host_id>`
--   SSH keys:        `dev.hrdtr.sheil.ssh_key.<key_name>`
CREATE TABLE IF NOT EXISTS credential (
    "service"         TEXT NOT NULL PRIMARY KEY,
    "encrypted_value" BLOB NOT NULL,
    "nonce"           BLOB NOT NULL,
    "created_at"      TEXT NOT NULL DEFAULT (datetime('now')),
    "updated_at"      TEXT NOT NULL DEFAULT (datetime('now'))
);
