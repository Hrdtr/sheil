# Plan: Credentials Manager in Sidebar

Status: Implemented
Date: 2026-08-15 (updated 2026-08-16)

## Objective

Add a **Credentials** view to the application sidebar, switchable alongside the
existing **Hosts** view via a vertical rail of square buttons. Credentials are
**first-class, reusable, named entities** of two kinds — **SSH keys** and
**passwords** — managed through a **single `credential` CRUD command set** whose
list endpoint is filterable by kind.

Underlying this is a **normalized `credential` table**: a `kind` discriminator
column (`key` | `password`), a UUIDv7 `id` primary key, and dedicated
`encrypted_value` / `key_passphrase_encrypted_value` columns. Hosts reference
credentials by `id`, so renames never touch host rows. New ids come from
`db::new_table_row_id()`; migrated ids are generated as UUIDv7 in SQL (see the
migration below).

No existing feature may change behavior when the **Hosts** view is active.

## Credential Model (target)

| Kind     | `kind`     | Identity | Label  | Extra column                                | Assign to host via |
| -------- | ---------- | -------- | ------ | ------------------------------------------- | ------------------ |
| SSH key  | `key`      | `id`     | `name` | `key_passphrase_encrypted_value` (nullable) | `host.key_id`      |
| Password | `password` | `id`     | `name` | —                                           | `host.password_id` |

- `id` is the stable primary key (UUIDv7); `name` is an editable display label.
- `encrypted_value`/`nonce` hold the key material or password.
- `key_passphrase_encrypted_value`/`key_passphrase_nonce` hold the optional key
  passphrase (NULL for passwords and unencrypted keys).
- `group`/`tags` mirror the host table (plaintext grouping + labels).
- `host.auth_method = 'key'` + `key_id`, or `'password'` + `password_id`. Both
  reference `credential.id` by convention (no enforced FK, matching the existing
  schema style).

## Schema & Migration

### New `credential` table

```sql
CREATE TABLE credential (
    "id"                   TEXT NOT NULL PRIMARY KEY,
    "name"                 TEXT NOT NULL,
    "kind"                 TEXT NOT NULL CHECK ("kind" IN ('key','password')),
    "encrypted_value"      BLOB NOT NULL,
    "nonce"                BLOB NOT NULL,
    "key_passphrase_encrypted_value" BLOB,
    "key_passphrase_nonce"     BLOB,
    "group"                TEXT,
    "tags"                 TEXT NOT NULL DEFAULT '[]',
    "created_at"           TEXT NOT NULL DEFAULT (datetime('now')),
    "updated_at"           TEXT NOT NULL DEFAULT (datetime('now'))
);
CREATE INDEX credential_kind_idx ON credential("kind");
CREATE INDEX credential_group_idx ON credential("group");
```

### Migration — single phase, pure SQL

One migration file `20260816003011_credential_refactor.sql`. No Rust promotion
step and no `promote_legacy_credentials` fn — everything runs in one SQL
transaction.

UUIDv7 ids are generated **in SQL** from primitives (`printf`/`random`/
`julianday`, all available since SQLite 3.8.6). `random()` is evaluated per row,
so each migrated row gets distinct random bits; `julianday('now')` is constant
within a statement, so its two 48-bit timestamp references split consistently.
New ids (rows created after this migration) still come from
`db::new_table_row_id()`; this SQL generator is migration-only.

Layout: `48-bit unix_ts_ms | ver=7 | 12-bit rand_a | var=10 | 62-bit rand_b`
→ `xxxxxxxx-xxxx-7xxx-[89ab]xxx-xxxxxxxxxxxx`.

```sql
-- Credential refactor: promote the legacy service-keyed `credential` table into
-- a typed table (kind = key | password) with UUIDv7 ids, and re-link hosts to
-- credential ids instead of key names / per-host passwords.

-- 1. Preserve legacy rows.
ALTER TABLE credential RENAME TO credential_legacy;

-- 2. New typed credential table.
CREATE TABLE credential (
    "id"                           TEXT NOT NULL PRIMARY KEY,
    "name"                         TEXT NOT NULL,
    "kind"                         TEXT NOT NULL CHECK ("kind" IN ('key','password')),
    "encrypted_value"              BLOB NOT NULL,
    "nonce"                        BLOB NOT NULL,
    "key_passphrase_encrypted_value" BLOB,
    "key_passphrase_nonce"           BLOB,
    "group"                        TEXT,
    "tags"                         TEXT NOT NULL DEFAULT '[]',
    "created_at"                   TEXT NOT NULL DEFAULT (datetime('now')),
    "updated_at"                   TEXT NOT NULL DEFAULT (datetime('now'))
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
```

Ordering matters: `credential_legacy` stays alive until the host `key_id` /
`password_id` links are done; only then is it dropped and `host` rebuilt without
`key_name`.

## Backend — one credential CRUD

### 1. Replace `secrets.rs` with a typed storage layer `credentials.rs`

`crypto.rs` (encrypt/decrypt primitives) is unchanged. The storage layer is
credential-aware and id-keyed:

```rust
#[derive(Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum CredentialKind { Key, Password }   // "key" | "password"

pub async fn create(pool, master_key, name: &str, kind: CredentialKind, value: &str,
                    key_passphrase_value: Option<&str>, group: Option<&str>, tags: &[String]) -> Result<String, String>
// id = db::new_table_row_id(); INSERT

pub async fn update(pool, master_key, id: &str, name: Option<&str>, value: Option<&str>,
                    key_passphrase_value: Option<&str>, group: Option<Option<String>>, tags: Option<Vec<String>>) -> Result<(), String>

pub async fn retrieve_value(pool, master_key, id: &str) -> Result<String, String>
pub async fn retrieve_key_passphrase_value(pool, master_key, id: &str) -> Result<Option<String>, String>
pub async fn list(pool, kind: Option<CredentialKind>) -> Result<Vec<CredentialRow>, String>
pub async fn delete(pool, id: &str) -> Result<(), String>
```

`list(None)` returns all rows; `list(Some(Key))` / `list(Some(Password))`
filters by the `kind` column.

### 2. New `tauri/src/commands/credentials.rs` — the single CRUD surface

```rust
#[derive(Deserialize)] #[serde(rename_all = "camelCase")]
pub struct CredentialInput {
    pub name: String,
    pub kind: CredentialKind,
    pub value: String,                 // key data or password
    pub key_passphrase_value: Option<String>,    // keys only
    pub group: Option<String>,
    #[serde(default)]
    pub tags: Vec<String>,
}

#[derive(Deserialize)] #[serde(rename_all = "camelCase")]
pub struct CredentialUpdate {
    pub name: Option<String>,
    pub value: Option<String>,
    pub key_passphrase_value: Option<String>,
    pub group: Option<Option<String>>,  // null clears
    pub tags: Option<Vec<String>>,
}

#[derive(Serialize)] #[serde(rename_all = "camelCase")]
pub struct CredentialInfo {
    pub id: String,
    pub name: String,
    pub kind: CredentialKind,
    pub key_type: Option<String>,      // keys only
    pub key_fingerprint: Option<String>,   // keys only
    pub group: Option<String>,
    pub tags: Vec<String>,
    pub created_at: String,
    pub updated_at: String,
}

#[command] pub async fn credential_create(input: CredentialInput) -> Result<CredentialInfo, String>
#[command] pub async fn credential_list(kind: Option<CredentialKind>) -> Result<Vec<CredentialInfo>, String>
#[command] pub async fn credential_update(id: String, update: CredentialUpdate) -> Result<CredentialInfo, String>
#[command] pub async fn credential_delete(id: String) -> Result<(), String>
#[command] pub async fn credential_resolve(id: String) -> Result<String, String>
```

Behavior:

- `credential_create` — validates by kind: for `key`, parse the key material
  (reusing `parse_private_key` from `ssh.rs`, made `pub(crate)`) and derive
  `key_type`/`key_fingerprint`; for `password`, require non-empty value. Stores via
  `credentials::create`.
- `credential_list` — `credentials::list(kind)`, deriving `key_type`/`key_fingerprint`
  for key rows (decrypt + parse), and returning `id`/`name`/`kind` only for
  passwords (never plaintext).
- `credential_update` — rename (`name`), new `value`/`key_passphrase_value`, and/or
  `group`/`tags` by id (re-validates keys).
- `credential_delete` — `DELETE FROM credential WHERE id = ?` then clear host
  references:
  ```sql
  UPDATE host SET key_id = NULL, auth_method = 'password' WHERE key_id = ?;
  UPDATE host SET password_id = NULL WHERE password_id = ?;
  ```
- `credential_resolve` — decrypt and return the plaintext value (used for
  password auth; keys are resolved server-side in `ssh_connect`).

### 3. `tauri/src/commands/ssh.rs` — remove key CRUD, keep sessions

- Remove `ssh_import_key`, `ssh_list_keys`, `ssh_delete_key`, `ImportedKeyInfo`,
  `store_ssh_key`, `retrieve_ssh_key`, and the service-prefix helpers.
- Keep `parse_private_key`, `key_fingerprint`, `try_parse_key_info` (mark
  `pub(crate)` for `credentials.rs`).
- `ssh_connect` key auth: `SshAuth::Key(credential_id)` →
  `credentials::retrieve_value(id)` + `credentials::retrieve_key_passphrase_value(id)` then
  `parse_private_key`.
- Remove the now-dead tests `imported_key_info_shape` and `ssh_key_service_format`
  (they reference the removed `ImportedKeyInfo` and service helpers).

### 4. `tauri/src/commands/hosts.rs` — id-based references, no credential logic

- `Host`/`HostInput`/`HostUpdate` gain `key_id` / `password_id` (`HostUpdate`:
  `Option<Option<String>>`); drop `key_name` and the inline `password` field.
- `host_create`/`host_update` persist `key_id` / `password_id` (clear on auth
  switch); no credential storage logic.
- `row_to_host`: `has_password = password_id.is_some()`.
- `host_resolve_password` is removed (password resolution moves to
  `credential_resolve`); also remove `host_password_service`,
  `store_host_password`, `host_has_password`, `delete_host_password` and their
  tests.
- `host_delete`: no per-host password cleanup (named passwords are shared).
- `host_export` / `host_import`:
  - export: serialize the **new** `Host` shape as-is (`key_id` / `password_id`,
    no passwords) — ids are exported verbatim, no name resolution.
  - import: detect the shape (new exports carry `key_id`/`password_id`; old
    exports carry `key_name`):
    - new shape: copy `key_id` / `password_id` directly; if the referenced
      `credential.id` does not exist in the target DB, set it to NULL.
    - old shape: map legacy `key_name` → `key_id` by looking up `credential` by
      `(name, 'key')`; if not found, NULL. (Old exports never carry passwords.)
- Update `hosts.rs` tests: `sample_input` and validation tests switch
  `key_name` → `key_id`; drop `password_service_is_namespaced` and the
  inline-password storage tests.

### 5. `tauri/src/commands.rs` + `tauri/src/lib.rs`

- `commands.rs`: add `pub mod credentials;`.
- `lib.rs`: `mod secrets;` → `mod credentials;`; remove
  `ssh_import_key`/`ssh_list_keys`/`ssh_delete_key`/`host_resolve_password` from
  the handler list; register `credential_create`, `credential_list`,
  `credential_update`, `credential_delete`, `credential_resolve`.

## Frontend — one credential namespace

### 6. `app/utils/commands.ts`

- Remove `ssh.importKey` / `ssh.listKeys` / `ssh.deleteKey` / `ImportedKey` and
  `host.resolvePassword`.
- Also remove `use-hosts.ts`'s `resolveHostPassword` (it wraps the removed
  `host.resolvePassword` and has no callers outside `use-hosts.ts`).
- `Host` gains `keyId: string | null` / `passwordId: string | null` (drops
  `keyName`); `HostInput`/`HostUpdate` drop `password`/`keyName`, gain
  `keyId`/`passwordId`.
- Add a single `credential` namespace:

```ts
type CredentialKind = 'key' | 'password';

interface Credential {
  id: string;
  name: string;
  kind: CredentialKind;
  keyType: string | null;
  keyFingerprint: string | null;
  group: string | null;
  tags: string[];
  createdAt: string;
  updatedAt: string;
}

interface CredentialInput {
  name: string;
  kind: CredentialKind;
  value: string;
  keyPassphraseValue?: string | null;
  group?: string | null;
  tags?: string[];
}

interface CredentialUpdate {
  name?: string | null;
  value?: string | null;
  keyPassphraseValue?: string | null;
  group?: string | null;
  tags?: string[];
}

const credential = {
  create: (input: CredentialInput) => invoke<Credential>('credential_create', { input }),
  list: (kind?: CredentialKind | null) =>
    invoke<Credential[]>('credential_list', { kind: kind ?? null }),
  update: (id: string, update: CredentialUpdate) =>
    invoke<Credential>('credential_update', { id, update }),
  delete: (id: string) => invoke('credential_delete', { id }),
  resolve: (id: string) => invoke<string>('credential_resolve', { id }),
};
```

`credential` joins the existing `export const commands` object. Types stay
module-local (like `Host`/`HostInput`); nothing is exported besides `commands`.

### 7. Replace `use-ssh-keys.ts` with unified `app/composables/use-credentials.ts`

One shared composable, mirroring `use-hosts.ts` (a single `useAsyncData` list +
`groupedCredentials`):

```ts
function _useCredentials() {
  const { hosts, refreshHosts } = useHosts();

  const {
    data: credentials,
    status: credentialsState,
    refresh: refreshCredentials,
  } = useAsyncData(() => commands.credential.list());

  const groupedCredentials = computed(() => {
    const groups = new Map<string, NonNullable<typeof credentials.value>>();
    for (const credential of credentials.value || []) {
      const key = credential.group || 'Other';
      if (!groups.has(key)) groups.set(key, []);
      groups.get(key)!.push(credential);
    }
    return [...groups.entries()].sort(([a], [b]) => {
      if (a === 'Other') return 1;
      if (b === 'Other') return -1;
      return a.localeCompare(b);
    });
  });

  const create: typeof commands.credential.create = async (input) => {
    const created = await commands.credential.create(input);
    await refreshCredentials();
    return created;
  };

  const update: typeof commands.credential.update = async (id, update) => {
    const updated = await commands.credential.update(id, update);
    await refreshCredentials();
    return updated;
  };

  const remove = async (id: string) => {
    await commands.credential.delete(id);
    await refreshCredentials();
    await refreshHosts();
  };

  const resolve = commands.credential.resolve;

  const assignToHost = async (
    hostId: string,
    credential: NonNullable<typeof credentials.value>[number],
  ) => {
    if (credential.kind === 'key') {
      await commands.host.update(hostId, {
        keyId: credential.id,
        authMethod: 'key',
        passwordId: null,
      });
    } else {
      await commands.host.update(hostId, {
        passwordId: credential.id,
        authMethod: 'password',
        keyId: null,
      });
    }
    await refreshHosts();
  };

  return {
    credentials,
    groupedCredentials,
    credentialsState,
    refreshCredentials,
    create,
    update,
    remove,
    resolve,
    assignToHost,
  };
}

export const useCredentials = createSharedComposable(_useCredentials);
```

- "used by N hosts" is derived in the components from `hosts` (via
  `host.keyId` / `host.passwordId`).
- `app/components/hosts.vue` `storedHost` uses `keyId`/`passwordId` (was
  `keyName`/`hasPassword`); the credential pickers set `formState.keyId` /
  `formState.passwordId`.

### 8. Replace `ssh-keys-manager.vue` with a single `app/components/credential-manager.vue`

One manager handles **both** kinds — no separate password manager:

- Props: `kind?: 'key' | 'password'` (filters the list; omit to show all),
  `selectable?: boolean`.
- Emits: `select` (`{ id, name }`), `imported`, `deleted`.
- Exposes `open`, `openAdd`, `close` (the add/import form adapts to `kind`).
- Add/import form: keys collect name + key data + `keyPassphraseValue`;
  passwords collect name + value; both collect `group`/`tags`.
- List rows show `name`, `keyType`/`keyFingerprint` (keys), `group`/`tags`, and
  "used by N hosts" (from `hosts` via `keyId`/`passwordId`).
- Backed by `useCredentials` (`create`/`update`/`remove`), filtered by `kind`.
- `use-ssh-keys.ts` is removed; its callers go through `useCredentials`.

### 9. New component — `app/components/credentials-panel.vue`

Inline sidebar list rendered from `groupedCredentials` (grouped by `group`, like
the host list); each row shows a key/password icon by `kind`, its name, tags,
and "used by N hosts" (computed from `hosts`). Row actions: **Assign to host…**
(host `Select` dialog → `assignToHost`), **Edit**, **Delete**. Exposes
`openImport` and `openAddPassword` for the header dropdown — both open the
unified `CredentialManager` (with `kind="key"` / `kind="password"`).

### 10. `app/components/hosts.vue` — password auth uses a picker

Replace the inline password `Input` (+ "Remove stored password" checkbox) with a
**"Select password…"** button opening `CredentialManager` (`kind="password"`,
`selectable`), mirroring "Select a key…" (`CredentialManager` `kind="key"`,
`selectable`). `formState` gains `passwordId` / `keyId`;
`createHost`/`updateHost` send `passwordId`/`keyId`/`authMethod`.

### 11. `app/components/app-sidebar.vue`

Vertical rail (Hosts/Credentials) + conditional list + view-aware "+":

- Hosts view → plain `createHost` button.
- Credentials view → `DropdownMenu`: **Add Password** / **Import SSH Key**.

### 12. `app/composables/use-sessions.ts` — `resolveAuth` via credentials

```ts
if (hostConfig.authMethod === 'key') {
  if (!hostConfig.keyId) throw new Error('No SSH key configured for this host');
  return { type: 'key', value: hostConfig.keyId };
}
const password = hostConfig.passwordId
  ? await commands.credential.resolve(hostConfig.passwordId).catch(() => '')
  : '';
return { type: 'password', value: password };
```

### 13. `app/components/quick-connect-dialog.vue`

- The `keyName` ref becomes a `{ id, name }` selection: `CredentialManager`
  (`kind="key"`, `selectable`) `@select` now receives `{ id, name }`; store both.
- Direct key auth passes the credential id: `{ type: 'key', value: selected.id }`
  (the button label still shows `selected.name`).
- Direct password auth is unchanged (plaintext entered at connect time via
  `connectDirect`).

## Non-Goals / Guarantees

- Do **not** change host list rendering, grouping, connect-on-click/double-click,
  edit/delete menus, tab bar, or session lifecycle.
- Do **not** reveal or copy plaintext passwords (write-only from the UI; list
  returns id+name only; `resolve` is used solely for connection).
- No password import from file (passwords are entered, not imported).
- `crypto.rs` and `db.rs` are unchanged; settings keep their own `setting` table.
- The key/password pickers used by the host form and quick connect keep working
  identically via the unified `CredentialManager`.

## Compatibility Checklist

Every existing feature is preserved end-to-end:

| Feature                                                         | After the change                                                                                   |
| --------------------------------------------------------------- | -------------------------------------------------------------------------------------------------- |
| Host CRUD (create/list/resolve/update/delete)                   | Same commands; `key_name`→`key_id`, inline `password`→`password_id`.                               |
| Connect (password auth)                                         | `resolveAuth` resolves the named password via `credential_resolve`.                                |
| Connect (key auth)                                              | `resolveAuth` passes `key_id`; `ssh_connect` retrieves by id.                                      |
| Quick connect (saved host / direct password)                    | Unchanged.                                                                                         |
| Quick connect (direct key)                                      | Updated to pass the key credential id.                                                             |
| SSH key import/list/delete (host form + quick connect)          | Same UX via unified `CredentialManager` + `credential` CRUD.                                       |
| Host import/export                                              | Preserved; export uses new shape (ids), import detects old (key_name) vs new (key_id/password_id). |
| Recent hosts / welcome screen                                   | Unchanged (host ids + names only).                                                                 |
| Settings → Data (export/import hosts, clear known hosts/recent) | Unchanged commands.                                                                                |
| Sessions, tab bar, reorder, terminal                            | Unchanged.                                                                                         |
| SFTP, port forwarding                                           | Unchanged.                                                                                         |
| AI engine, settings, known hosts                                | Unchanged.                                                                                         |
| Credential encryption (AES-256-GCM, master key)                 | `crypto.rs` unchanged; legacy ciphertext copied verbatim.                                          |

## Verification

1. `pnpm run typecheck`, `pnpm run lint`, `pnpm run test`.
2. Rust tests: `credentials` create/update/retrieve/list(filter)/delete by id;
   `credential_*` commands (key validation on create, list filtering, delete
   clears host refs, resolve returns plaintext); `host_export`/`host_import`
   new-shape id round-trip + old-shape (`key_name`) conversion with
   dangling→NULL; migration test (seed legacy `sheil.ssh_key.*` /
   `sheil.host_password.*` rows, run migrations, assert every `credential.id`
   parses as UUIDv7 and `host.key_id`/`password_id` link correctly).
3. Manual:
   - Migration on an existing DB: old keys keep their passphrase; old host
     passwords appear as named password credentials; hosts still connect.
   - Credentials view: "+" dropdown → Add Password / Import SSH Key.
   - Add a password → listed; assign it to two hosts → both connect; "used by 2
     hosts" shown.
   - Rename a password → both hosts still resolve (id-based reference).
   - Delete a password → hosts fall back to empty password.
   - Assign a key → host switches to key auth and old password link cleared.
   - Host form: password auth uses the picker; key auth unchanged.
   - Quick connect: saved host + direct password + direct key all connect.
   - Export hosts (new shape) → import preserves key_id/password_id; missing ids
     become NULL; an old-shape export with key_name imports by resolving the name.
   - Hosts view unchanged.

## Open Questions

- Should `name` be unique (e.g. `UNIQUE(name)` or `UNIQUE(name, kind)`) to
  preserve today's "import overwrites by name" behavior, or allow duplicates
  now that `id` is the identity? Default plan: no uniqueness; create always adds.
- Keep a "type a new password inline" shortcut in the host form that auto-creates
  a named credential? Default plan: no.
- Persist selected sidebar view across restarts? Default plan: no.
- Rail icon: `MonitorIcon` vs `ServerIcon` for Hosts.
