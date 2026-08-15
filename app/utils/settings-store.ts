import { commands } from './commands';

interface SettingEntry {
  key: string;
  value: string;
  defaultValue: string;
  valueType: string;
  createdAt: string;
  updatedAt: string;
}

interface SettingInput {
  key: string;
  value: string;
}

function camelToSnake(value: string): string {
  return value.replace(/([a-z0-9])([A-Z])/g, '$1_$2').toLowerCase();
}

function snakeToCamel(value: string): string {
  return value.replace(/_([a-z0-9])/g, (_, c: string) => c.toUpperCase());
}

function coerce(value: string, type: string): unknown {
  switch (type) {
    case 'number':
      return Number(value);
    case 'boolean':
      return value === 'true';
    case 'null':
      return null;
    case 'json':
      try {
        return JSON.parse(value);
      } catch {
        return {};
      }
    default:
      return value;
  }
}

function encodeValue(value: unknown): string {
  if (value === null) return '';
  const type = typeof value;
  if (type === 'number' || type === 'boolean' || type === 'string') return String(value);
  return JSON.stringify(value);
}

function flattenSettings(obj: Record<string, unknown>, namespace: string): SettingInput[] {
  const entries: SettingInput[] = [];
  for (const [prop, value] of Object.entries(obj)) {
    entries.push({ key: `${namespace}.${camelToSnake(prop)}`, value: encodeValue(value) });
  }
  return entries;
}

function setByPath(target: Record<string, unknown>, path: string, value: unknown): void {
  const segments = path.split('.').map(snakeToCamel);
  let node = target;
  for (let i = 0; i < segments.length - 1; i++) {
    const segment = segments[i]!;
    if (typeof node[segment] !== 'object' || node[segment] === null) {
      node[segment] = {};
    }
    node = node[segment] as Record<string, unknown>;
  }
  const last = segments[segments.length - 1]!;
  node[last] = value;
}

function unflattenNamespace<T>(
  entries: Record<string, SettingEntry>,
  namespace: string,
  field: 'value' | 'defaultValue',
): T {
  const prefix = `${namespace}.`;
  const result: Record<string, unknown> = {};
  for (const entry of Object.values(entries)) {
    if (!entry.key.startsWith(prefix)) continue;
    const relativeKey = entry.key.slice(prefix.length);
    const raw = field === 'value' ? entry.value : entry.defaultValue;
    setByPath(result, relativeKey, coerce(raw, entry.valueType));
  }
  return result as T;
}

async function reload(): Promise<void> {
  const entries = useState<Record<string, SettingEntry>>('settings:entries', () => ({}));
  const all = await commands.settings.getAll();
  const map: Record<string, SettingEntry> = {};
  for (const entry of all) map[entry.key] = entry;
  entries.value = map;
}

export const settingsStore = {
  async init(): Promise<void> {
    await reload();
  },

  applyLocal(namespace: string, obj: Record<string, unknown>): void {
    const entries = useState<Record<string, SettingEntry>>('settings:entries', () => ({}));
    for (const input of flattenSettings(obj, namespace)) {
      const existing = entries.value[input.key];
      if (existing) entries.value[input.key] = { ...existing, value: input.value };
    }
  },

  async persist(namespace: string, obj: Record<string, unknown>): Promise<void> {
    await commands.settings.setMany(flattenSettings(obj, namespace));
  },

  async resetNamespaces(namespaces: string[]): Promise<void> {
    const entries = useState<Record<string, SettingEntry>>('settings:entries', () => ({}));
    const keys = Object.values(entries.value)
      .filter((entry) => namespaces.some((ns) => entry.key.startsWith(`${ns}.`)))
      .map((entry) => entry.key);
    await commands.settings.resetMany(keys);
    await reload();
  },

  namespaceSettings<T>(namespace: string): T {
    const entries = useState<Record<string, SettingEntry>>('settings:entries', () => ({}));
    return unflattenNamespace<T>(entries.value, namespace, 'value');
  },

  namespaceDefaults<T>(namespace: string): T {
    const entries = useState<Record<string, SettingEntry>>('settings:entries', () => ({}));
    return unflattenNamespace<T>(entries.value, namespace, 'defaultValue');
  },
};
