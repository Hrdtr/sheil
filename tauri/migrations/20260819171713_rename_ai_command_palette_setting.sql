-- Rename the AI command palette toggle to command generator, preserving the
-- user's current value. Runs before `seed_settings`, so the renamed row is
-- kept (seeding uses ON CONFLICT("key") DO NOTHING).
UPDATE setting
SET "key" = 'ai.command_generator_enabled'
WHERE "key" = 'ai.command_palette_enabled'
  AND NOT EXISTS (
    SELECT 1 FROM setting WHERE "key" = 'ai.command_generator_enabled'
  );

-- Drop any leftover old row (only present if the new key already existed).
DELETE FROM setting WHERE "key" = 'ai.command_palette_enabled';
