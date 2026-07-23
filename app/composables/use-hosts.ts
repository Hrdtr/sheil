/**
 * Shared composable providing reactive access to SSH host configurations.
 *
 * Wraps the Tauri host commands with automatic list refresh after
 * mutations (create, update, delete). Use {@link useHosts} — the
 * shared singleton — rather than calling this directly.
 */
function _useHosts() {
  const {
    data: hosts,
    status: hostsState,
    refresh: refreshHosts,
  } = useAsyncData(() => commands.host.list());

  const groupedHosts = computed(() => {
    const groups = new Map<string, NonNullable<typeof hosts.value>>();
    for (const host of hosts.value || []) {
      const key = host.group || 'Other';
      if (!groups.has(key)) groups.set(key, []);
      groups.get(key)!.push(host);
    }
    return [...groups.entries()].sort(([a], [b]) => {
      if (a === 'Other') return 1;
      if (b === 'Other') return -1;
      return a.localeCompare(b);
    });
  });

  /**
   * Create a new SSH host configuration.
   * Automatically refreshes the host list on success.
   */
  const createHost: typeof commands.host.create = async (payload) => {
    const createdHost = await commands.host.create(payload);
    await refreshHosts();
    return createdHost;
  };

  /**
   * Fetch a single host configuration by id.
   */
  const resolveHost: typeof commands.host.resolve = async (id) => {
    const resolvedHost = await commands.host.resolve(id);
    return resolvedHost;
  };

  /**
   * Update an existing SSH host configuration.
   * Automatically refreshes the host list on success.
   */
  const updateHost: typeof commands.host.update = async (id, payload) => {
    const updatedHost = await commands.host.update(id, payload);
    await refreshHosts();
    return updatedHost;
  };

  /**
   * Delete an SSH host configuration.
   * Automatically refreshes the host list on success.
   */
  const deleteHost: typeof commands.host.delete = async (host) => {
    const deletedHost = await commands.host.delete(host);
    await refreshHosts();
    return deletedHost;
  };

  /**
   * Resolve the stored password for a host + username combination.
   *
   * The password is decrypted from secure storage via the Rust backend.
   * This does **not** refresh the host list since no configuration changes.
   */
  const resolveHostPassword: typeof commands.host.resolvePassword = async (id) => {
    const password = await commands.host.resolvePassword(id);
    return password;
  };

  return {
    hosts,
    groupedHosts,
    hostsState,
    refreshHosts,
    createHost,
    resolveHost,
    updateHost,
    deleteHost,
    resolveHostPassword,
  };
}

export const useHosts = createSharedComposable(_useHosts);
