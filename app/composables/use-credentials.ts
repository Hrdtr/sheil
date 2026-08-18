type Credential = Awaited<ReturnType<typeof commands.credential.list>>[number];

/**
 * Shared composable providing reactive access to credentials (SSH keys and
 * passwords).
 *
 * Wraps the unified `credential` commands with automatic list refresh after
 * mutations (create, update, delete). Use {@link useCredentials} — the shared
 * singleton — rather than calling commands directly.
 */
function _useCredentials() {
  const { refreshHosts } = useHosts();

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

  // ── Filtering ────────────────────────────────────────────────────────────

  /**
   * Filter credentials by free-text query (name, command, description, tags)
   * and optional group / scope constraints.
   */
  function filterCredentials(options: {
    query?: string;
    group?: string | null;
    kind?: string | null;
    keyType?: string | null;
  }): Credential[] {
    const query = options.query?.trim().toLowerCase() ?? '';
    const group = options.group ?? null;
    const kind = options.kind ?? null;
    const keyType = options.keyType ?? null;

    return (credentials.value ?? []).filter((credential) => {
      if (group && credential.group !== group) return false;
      if (kind && credential.kind !== kind) return false;
      if (keyType && credential.keyType !== keyType) return false;
      if (!query) return true;

      const haystack = [credential.name, ...credential.tags].join(' ').toLowerCase();
      return haystack.includes(query);
    });
  }

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
    filterCredentials,
    create,
    update,
    remove,
    resolve,
    assignToHost,
  };
}

export const useCredentials = createSharedComposable(_useCredentials);
