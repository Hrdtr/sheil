-- Credential refactor: promote the legacy service-keyed `credential` table into
-- a typed table (kind = key | password) with UUIDv7 ids, and re-link hosts to
-- credential ids instead of key names / per-host passwords.
--
-- Single phase, pure SQL — no Rust promotion step. UUIDv7 ids are generated in
-- SQL from primitives (`printf`/`random`/`julianday`, available since SQLite
-- 3.8.6). `random()` is evaluated per row so each migrated row gets distinct
-- random bits; `julianday('now')` is constant within a statement, so its two
-- 48-bit timestamp references split consistently. New ids (rows created after
-- this migration) come from `db::new_table_row_id()`.
--
-- Layout: 48-bit unix_ts_ms | ver=7 | 12-bit rand_a | var=10 | 62-bit rand_b
--       -> xxxxxxxx-xxxx-7xxx-[89ab]xxx-xxxxxxxxxxxx

-- 1. Preserve legacy rows.
ALTER TABLE credential RENAME TO credential_legacy;

-- 2. New typed credential table.
CREATE TABLE credential (
    "id"                             TEXT NOT NULL PRIMARY KEY,
    "name"                           TEXT NOT NULL,
    "kind"                           TEXT NOT NULL CHECK ("kind" IN ('key','password')),
    "encrypted_value"                BLOB NOT NULL,
    "nonce"                          BLOB NOT NULL,
    "key_passphrase_encrypted_value" BLOB,
    "key_passphrase_nonce"           BLOB,
    "group"                          TEXT,
    "tags"                           TEXT NOT NULL DEFAULT '[]',
    "created_at"                     TEXT NOT NULL DEFAULT (datetime('now')),
    "updated_at"                     TEXT NOT NULL DEFAULT (datetime('now'))
);
CREATE INDEX credential_kind_idx  ON credential("kind");
CREATE INDEX credential_group_idx ON credential("group");

-- 3. Host reference columns (key_name kept for the name→id link below).
ALTER TABLE host ADD COLUMN "key_id" TEXT;
ALTER TABLE host ADD COLUMN "password_id" TEXT;

-- 4a. SSH keys — merge the '.passphrase' row into a column.
INSERT INTO credential
  ("id","name","kind","encrypted_value","nonce",
   "key_passphrase_encrypted_value","key_passphrase_nonce",
   "group","tags","created_at","updated_at")
SELECT
  printf('%08x-%04x-7%03x-%04x-%012x',
    CAST((julianday('now') - 2440587.5) * 86400000 AS INTEGER) >> 16,
    CAST((julianday('now') - 2440587.5) * 86400000 AS INTEGER) & 0xFFFF,
    random() & 0xFFF,
    (random() & 0x3FFF) | 0x8000,
    random() & 0xFFFFFFFFFFFF),
  substr(k."service", length('sheil.ssh_key.') + 1),
  'key',
  k."encrypted_value", k."nonce",
  p."encrypted_value", p."nonce",
  NULL, '[]', k."created_at", k."updated_at"
FROM credential_legacy AS k
LEFT JOIN credential_legacy AS p
  ON p."service" = 'sheil.ssh_key.'
      || substr(k."service", length('sheil.ssh_key.') + 1) || '.passphrase'
WHERE k."service" LIKE 'sheil.ssh_key.%'
  AND k."service" NOT LIKE '%.passphrase';

-- 4b. Host passwords — promote each to a named password credential.
INSERT INTO credential
  ("id","name","kind","encrypted_value","nonce",
   "key_passphrase_encrypted_value","key_passphrase_nonce",
   "group","tags","created_at","updated_at")
SELECT
  printf('%08x-%04x-7%03x-%04x-%012x',
    CAST((julianday('now') - 2440587.5) * 86400000 AS INTEGER) >> 16,
    CAST((julianday('now') - 2440587.5) * 86400000 AS INTEGER) & 0xFFFF,
    random() & 0xFFF,
    (random() & 0x3FFF) | 0x8000,
    random() & 0xFFFFFFFFFFFF),
  h."name" || ' (' || substr(h."id", 1, 8) || ')',
  'password',
  c."encrypted_value", c."nonce",
  NULL, NULL,
  NULL, '[]', c."created_at", c."updated_at"
FROM credential_legacy AS c
JOIN host AS h ON c."service" = 'sheil.host_password.' || h."id";

-- 5. Re-link hosts by credential NAME → id.
UPDATE host
SET "key_id" = (
  SELECT c."id" FROM credential c
  WHERE c."kind" = 'key' AND c."name" = host."key_name"
)
WHERE host."key_name" IS NOT NULL;

UPDATE host
SET "password_id" = (
  SELECT c."id" FROM credential c
  WHERE c."kind" = 'password'
    AND c."name" = host."name" || ' (' || substr(host."id", 1, 8) || ')'
);

-- 6. Drop legacy data.
DROP TABLE credential_legacy;

-- 7. Rebuild host without "key_name" (universal pattern; no DROP COLUMN).
CREATE TABLE host_new (
    "id"           TEXT PRIMARY KEY,
    "name"         TEXT NOT NULL,
    "host"         TEXT NOT NULL,
    "port"         INTEGER NOT NULL DEFAULT 22,
    "username"     TEXT NOT NULL,
    "protocol"     TEXT NOT NULL DEFAULT 'ssh',
    "group"        TEXT,
    "auth_method"  TEXT NOT NULL DEFAULT 'password',
    "key_id"       TEXT,
    "password_id"  TEXT,
    "tags"         TEXT NOT NULL DEFAULT '[]',
    "created_at"   TEXT NOT NULL DEFAULT (datetime('now')),
    "updated_at"   TEXT NOT NULL DEFAULT (datetime('now'))
);

INSERT INTO host_new
    ("id","name","host","port","username","protocol","group","auth_method",
     "key_id","password_id","tags","created_at","updated_at")
SELECT
    "id","name","host","port","username","protocol","group","auth_method",
    "key_id","password_id","tags","created_at","updated_at"
FROM host;

DROP TABLE host;
ALTER TABLE host_new RENAME TO host;

CREATE INDEX host_group_idx       ON host("group");
CREATE INDEX host_name_idx        ON host("name");
CREATE INDEX host_key_id_idx      ON host("key_id");
CREATE INDEX host_password_id_idx ON host("password_id");
