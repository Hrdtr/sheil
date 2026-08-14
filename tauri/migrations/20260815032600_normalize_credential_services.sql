-- Normalize credential `service` namespaces to a consistent `sheil.*` scheme.
--
-- Before:
--   Host passwords:      sheil.host.<host_id>
--   SSH private keys:    dev.hrdtr.sheil.ssh_key.<name>
--   SSH key passphrases: dev.hrdtr.sheil.ssh_key.<name>.passphrase
-- After:
--   Host passwords:      sheil.host_password.<host_id>
--   SSH private keys:    sheil.ssh_key.<name>
--   SSH key passphrases: sheil.ssh_key.<name>.passphrase
--
-- Values are already AES-256-GCM ciphertext, so only the `service` column is
-- rewritten — no re-encryption is required.

-- Host passwords: sheil.host.<id> -> sheil.host_password.<id>
UPDATE credential
SET "service" = 'sheil.host_password.' || substr("service", length('sheil.host.') + 1),
    "updated_at" = datetime('now')
WHERE "service" LIKE 'sheil.host.%';

-- SSH keys and passphrases:
--   dev.hrdtr.sheil.ssh_key.<name>[.passphrase] -> sheil.ssh_key.<name>[.passphrase]
UPDATE credential
SET "service" = 'sheil.ssh_key.' || substr("service", length('dev.hrdtr.sheil.ssh_key.') + 1),
    "updated_at" = datetime('now')
WHERE "service" LIKE 'dev.hrdtr.sheil.ssh_key.%';
