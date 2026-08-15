-- App settings key-value store.
--
-- Each row is a single setting. `key` is a dot-separated snake_case path into
-- the nested settings object (e.g. `terminal.appearance.font_size`). Rows are
-- seeded at startup from Rust (`seed_settings`) with their `default_value`;
-- `value` holds the user's current value. `value_type` drives how the TEXT
-- value is coerced back into its JS type on read.

CREATE TABLE IF NOT EXISTS setting (
    "key"           TEXT NOT NULL PRIMARY KEY,
    "value"         TEXT NOT NULL,
    "default_value" TEXT NOT NULL,
    "value_type"    TEXT NOT NULL DEFAULT 'string'
                    CHECK ("value_type" IN ('string','number','boolean','null','json')),
    "created_at"    TEXT NOT NULL DEFAULT (datetime('now')),
    "updated_at"    TEXT NOT NULL DEFAULT (datetime('now'))
);
