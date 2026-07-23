type SftpEntry = Awaited<ReturnType<typeof commands.sftp.listDir>>[number];

function _useSftp() {
  // --- State ---
  const pathBySession = reactive(new Map<string, string>());
  const entries = ref<SftpEntry[]>([]);
  const loading = ref(false);
  const error = ref<string | null>(null);
  const panelOpen = ref(false);

  // --- Session tracking ---
  const connected = ref(false);
  const lastSessionId = ref<string | null>(null);
  let opening = false;

  /** The current path for the active SFTP session. */
  const currentPath = computed({
    get: () => pathBySession.get(lastSessionId.value ?? '') ?? '/',
    set: (val) => {
      if (lastSessionId.value) pathBySession.set(lastSessionId.value, val);
    },
  });

  /**
   * Open an SFTP session on the given SSH session id.
   * Restores the last-browsed path if available, otherwise resolves home.
   */
  async function open(sessionId: string): Promise<void> {
    if (connected.value && lastSessionId.value === sessionId) return;
    if (opening) return;
    opening = true;
    try {
      await commands.sftp.connect(sessionId);
      connected.value = true;
      lastSessionId.value = sessionId;
      // If no saved path for this session, resolve home
      if (!pathBySession.has(sessionId)) {
        try {
          const home = await commands.sftp.canonicalize(sessionId, '.');
          pathBySession.set(sessionId, home);
        } catch {
          pathBySession.set(sessionId, '/');
        }
      }
      await refresh(sessionId, currentPath.value);
    } finally {
      opening = false;
    }
  }

  /** Close the SFTP session and reset state. */
  async function close(sessionId: string): Promise<void> {
    try {
      await commands.sftp.disconnect(sessionId);
    } catch {
      // Best-effort
    }
    connected.value = false;
    lastSessionId.value = null;
    loading.value = true;
    entries.value = [];
  }

  /** Refresh the current directory listing. */
  async function refresh(sessionId: string, path?: string): Promise<void> {
    const targetPath = path ?? currentPath.value;
    loading.value = true;
    error.value = null;
    try {
      entries.value = await commands.sftp.listDir(sessionId, targetPath);
      currentPath.value = targetPath;
    } catch (e) {
      error.value = e instanceof Error ? e.message : String(e);
      entries.value = [];
    } finally {
      loading.value = false;
    }
  }

  /** Navigate into a directory. */
  async function navigate(sessionId: string, path: string): Promise<void> {
    await refresh(sessionId, path);
  }

  /** Navigate up one level. */
  async function goUp(sessionId: string): Promise<void> {
    const parent = currentPath.value.split('/').slice(0, -1).join('/') || '/';
    await refresh(sessionId, parent);
  }

  /** Create a new directory. */
  async function createDirectory(sessionId: string, name: string): Promise<void> {
    const path = `${currentPath.value.replace(/\/$/, '')}/${name}`;
    await commands.sftp.createDir(sessionId, path);
    await refresh(sessionId);
  }

  /** Delete a file or empty directory. */
  async function deleteEntry(sessionId: string, entry: SftpEntry): Promise<void> {
    if (entry.isDir) {
      await commands.sftp.removeDir(sessionId, entry.path);
    } else {
      await commands.sftp.removeFile(sessionId, entry.path);
    }
    await refresh(sessionId);
  }

  /** Rename a file or directory. */
  async function renameEntry(sessionId: string, oldPath: string, newName: string): Promise<void> {
    const parts = oldPath.split('/');
    parts[parts.length - 1] = newName;
    const newPath = parts.join('/');
    await commands.sftp.rename(sessionId, oldPath, newPath);
    await refresh(sessionId);
  }

  /** Toggle the SFTP panel visibility. */
  function togglePanel(): void {
    panelOpen.value = !panelOpen.value;
  }

  return {
    currentPath,
    entries,
    loading,
    error,
    panelOpen,
    connected,
    open,
    close,
    refresh,
    navigate,
    goUp,
    createDirectory,
    deleteEntry,
    renameEntry,
    togglePanel,
  };
}

export const useSftp = createSharedComposable(_useSftp);
