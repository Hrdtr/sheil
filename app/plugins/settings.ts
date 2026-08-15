const MIGRATED_FLAG = 'sheil:settings:migrated';

const MIGRATIONS = [
  { storageKey: 'terminal-settings:appearance', namespace: 'terminal.appearance' },
  { storageKey: 'terminal-settings:behavior', namespace: 'terminal.behavior' },
  { storageKey: 'ai-settings', namespace: 'ai' },
  { storageKey: 'ssh-settings', namespace: 'ssh' },
];

export default defineNuxtPlugin(async () => {
  try {
    await settingsStore.init();
  } catch (error) {
    console.error('failed to load settings:', error);
    return;
  }

  if (localStorage.getItem(MIGRATED_FLAG)) return;

  // Legacy `terminal-settings:appearance` only stored a `colorSchemeId`, which
  // no longer maps to a seeded key — so the color scheme reverts to the
  // seeded default (Catppuccin Mocha). Everything else migrates 1:1.
  for (const { storageKey, namespace } of MIGRATIONS) {
    const raw = localStorage.getItem(storageKey);
    if (!raw) continue;
    try {
      const obj = JSON.parse(raw) as Record<string, unknown>;
      settingsStore.applyLocal(namespace, obj);
      await settingsStore.persist(namespace, obj);
    } catch {
      // ignore malformed legacy entries
    }
  }

  localStorage.setItem(MIGRATED_FLAG, '1');
});
