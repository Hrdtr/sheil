/**
 * Persisted list of recently connected host IDs.
 * Automatically prunes entries for hosts that no longer exist.
 */
function _useRecentHosts() {
  const recentHostIds = useLocalStorage<string[]>('sheil:recent-hosts', []);

  const { hosts } = useHosts();

  /** Add a host to the top of the recent list. */
  function push(hostId: string) {
    recentHostIds.value = [hostId, ...recentHostIds.value.filter((id) => id !== hostId)].slice(
      0,
      10,
    );
  }

  /** Remove a host from the recent list. */
  function remove(hostId: string) {
    recentHostIds.value = recentHostIds.value.filter((id) => id !== hostId);
  }

  /** Clear all recent hosts. */
  function clear() {
    recentHostIds.value = [];
  }

  /** Recent hosts that still exist in the host list, most recent first. */
  const recentHosts = computed(() => {
    const ids = new Set(recentHostIds.value);
    return (hosts.value ?? [])
      .filter((host) => ids.has(host.id))
      .sort((a, b) => recentHostIds.value.indexOf(a.id) - recentHostIds.value.indexOf(b.id));
  });

  // Prune deleted hosts from the persisted list.
  watch(hosts, (list) => {
    const valid = new Set((list ?? []).map((host) => host.id));
    recentHostIds.value = recentHostIds.value.filter((id) => valid.has(id));
  });

  return { recentHosts, push, remove, clear };
}

export const useRecentHosts = createSharedComposable(_useRecentHosts);
