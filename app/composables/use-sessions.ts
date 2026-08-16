import { listen } from '@tauri-apps/api/event';

type SessionState = 'connecting' | 'connected' | 'error' | 'disconnected';

/** An open terminal tab — every tab IS an active SSH session. */
interface Session {
  /** Unique tab identifier (separate from the backend SSH session id). */
  tabId: string;
  /** Id of the host configuration this session was launched from. */
  hostId: string;
  /** Display name for the tab (resolved from host config on connect). */
  hostName: string;
  /** Dynamic title from terminal OSC title sequence (e.g. cwd, shell). */
  title: string | null;
  /** SSH session id from the Rust backend, `null` until connected. */
  sshSessionId: string | null;
  /** Current connection state. */
  state: SessionState;
  /** Error message when `state` is `'error'`. */
  error: string | null;
}

/** Authentication payload for the Rust SSH connect command. */
type SshAuth = { type: 'password'; value: string } | { type: 'key'; value: string };

/**
 * SSH session + tab lifecycle composable.
 *
 * Manages the collection of open terminal tabs (each bound to an active or
 * in-progress SSH connection). Session state and tab state are **the same
 * thing** — closing a tab means disconnecting its session, and every session
 * has exactly one tab. There is no tab without a session.
 *
 * Wrapped with {@link createSharedComposable} so the function body
 * runs once — `tabCounter`, computed refs, and {@link useState}
 * keys are all shared across components.
 *
 * @returns
 * - `sessions` — reactive array of open sessions/tabs
 * - `activeTabId` — id of the focused tab (`null` when none open)
 * - `activeSession` — the currently focused session (or `null`)
 * - `connect(hostId)` — resolve the host, open a new tab, and initiate SSH
 * - `disconnect(tabId)` — close SSH channel and remove the tab
 * - `switchTab(tabId)` — focus a different tab
 */
function _useSessions() {
  let tabCounter = 0;
  function newTabId(): string {
    tabCounter += 1;
    return `tab_${tabCounter}`;
  }

  const { keepaliveInterval, connectTimeout } = useSshSettings();

  // ---- State ----

  const sessions = useState<Session[]>('sessions', () => []);
  const activeTabId = useState<string | null>('sessions:active-tab-id', () => null);

  const activeSession = computed<Session | null>(() => {
    return sessions.value.find((session) => session.tabId === activeTabId.value) ?? null;
  });

  // ---- Auth helpers ----

  /**
   * Resolve authentication payload for a given host config — either a
   * key credential id or a decrypted password from secure storage.
   */
  async function resolveAuth(
    hostConfig: Awaited<ReturnType<typeof commands.host.resolve>>,
  ): Promise<SshAuth> {
    if (hostConfig.authMethod === 'key') {
      if (!hostConfig.keyId) {
        throw new Error('No SSH key configured for this host');
      }
      return { type: 'key', value: hostConfig.keyId };
    }

    const password = hostConfig.passwordId
      ? await commands.credential.resolve(hostConfig.passwordId).catch(() => '')
      : '';
    return { type: 'password', value: password };
  }

  // ---- Public API ----

  /**
   * Resolve a host by id, open a new tab, and initiate an SSH connection.
   *
   * The host is resolved from the database first so the tab only stores
   * the `hostId` rather than a potentially stale full config object. The
   * tab is created immediately in `'connecting'` state so the UI can
   * show feedback before the connection completes.
   *
   * On success the tab transitions to `'connected'`; on failure it
   * transitions to `'error'` and the error is rethrown.
   *
   * @param hostId - Id of the host configuration to connect to.
   * @param insertIndex - Optional index to insert the tab at (for reconnection).
   * @returns The backend SSH session id.
   */
  const connect = async (hostId: string, insertIndex?: number): Promise<string> => {
    const hostConfig = await commands.host.resolve(hostId);
    const tabId = newTabId();

    useRecentHosts().push(hostId);

    const session: Session = {
      tabId,
      hostId,
      hostName: hostConfig.name,
      title: null,
      sshSessionId: null,
      state: 'connecting',
      error: null,
    };

    if (insertIndex !== undefined && insertIndex >= 0 && insertIndex <= sessions.value.length) {
      sessions.value.splice(insertIndex, 0, session);
    } else {
      sessions.value.push(session);
    }
    activeTabId.value = tabId;

    try {
      const sshSessionId = await commands.ssh.connect(
        hostConfig.host,
        hostConfig.port,
        hostConfig.username,
        await resolveAuth(hostConfig),
        keepaliveInterval.value || null,
        connectTimeout.value || null,
      );

      sessions.value = sessions.value.map((session) =>
        session.tabId === tabId ? { ...session, sshSessionId, state: 'connected' } : session,
      );
      return sshSessionId;
    } catch (err) {
      const message = err instanceof Error ? err.message : String(err);
      sessions.value = sessions.value.map((session) =>
        session.tabId === tabId ? { ...session, state: 'error', error: message } : session,
      );
      throw err;
    }
  };

  /**
   * Disconnect an SSH session and remove its tab.
   *
   * The SSH channel is closed on the backend; the PTY is cleaned up by
   * {@link TerminalView}'s `onUnmounted` hook. This is best-effort — if
   * the SSH session is already gone, the disconnect call is skipped.
   *
   * When the removed tab was the active one, focus shifts to the last
   * remaining tab (or `null` if none remain).
   */
  const disconnect = async (tabId: string): Promise<void> => {
    const session = sessions.value.find((session) => session.tabId === tabId);
    if (!session) return;

    if (session.sshSessionId && session.state === 'connected') {
      try {
        await commands.ssh.disconnect(session.sshSessionId);
      } catch {
        // Best-effort — the session may already be gone.
      }
    }

    sessions.value = sessions.value.filter((session) => session.tabId !== tabId);
    if (activeTabId.value === tabId) {
      activeTabId.value =
        sessions.value.length > 0 ? sessions.value[sessions.value.length - 1]!.tabId : null;
    }
  };

  /**
   * Focus an existing tab for a host, or open a new connection when none exists.
   *
   * @param hostId - Id of the host configuration to connect to.
   * @returns The existing backend SSH session id, or the new one from {@link connect}.
   */
  const focusOrConnect = async (hostId: string): Promise<string | null> => {
    const existing = sessions.value.find((session) => session.hostId === hostId);
    if (existing) {
      switchTab(existing.tabId);
      return existing.sshSessionId;
    }
    return connect(hostId);
  };

  /**
   * Activate a tab by id. No-op if the id doesn't match an existing tab.
   */
  const switchTab = (tabId: string): void => {
    if (sessions.value.some((session) => session.tabId === tabId)) {
      activeTabId.value = tabId;
    }
  };

  /** Update the dynamic title for a session (called from Terminal onTitleChange). */
  const setTitle = (tabId: string, title: string): void => {
    sessions.value = sessions.value.map((s) => (s.tabId === tabId ? { ...s, title } : s));
  };

  /** Reorder a session tab by moving it from `fromIndex` to `toIndex`. */
  const reorder = (fromIndex: number, toIndex: number): void => {
    const list = [...sessions.value];
    const [item] = list.splice(fromIndex, 1);
    list.splice(toIndex, 0, item!);
    sessions.value = list;
  };

  /**
   * Connect to a host directly without a saved host config.
   * Used by the Quick Connect dialog for on-the-fly connections.
   */
  const connectDirect = async (
    host: string,
    port: number,
    username: string,
    auth: SshAuth,
  ): Promise<string> => {
    const tabId = newTabId();

    sessions.value.push({
      tabId,
      hostId: '',
      hostName: `${username}@${host}`,
      title: null,
      sshSessionId: null,
      state: 'connecting',
      error: null,
    });
    activeTabId.value = tabId;

    try {
      const sshSessionId = await commands.ssh.connect(
        host,
        port,
        username,
        auth,
        keepaliveInterval.value || null,
        connectTimeout.value || null,
      );

      sessions.value = sessions.value.map((session) =>
        session.tabId === tabId ? { ...session, sshSessionId, state: 'connected' } : session,
      );
      return sshSessionId;
    } catch (err) {
      const message = err instanceof Error ? err.message : String(err);
      sessions.value = sessions.value.map((session) =>
        session.tabId === tabId ? { ...session, state: 'error', error: message } : session,
      );
      throw err;
    }
  };

  return {
    sessions,
    activeTabId,
    activeSession,
    connect,
    disconnect,
    focusOrConnect,
    switchTab,
    setTitle,
    reorder,
    connectDirect,
  };
}

// Global listener: detect when the SSH server closes the connection
// (e.g. user typed `exit`) and mark the session as disconnected.
if (import.meta.client) {
  listen<string>('ssh-exit', (event) => {
    const sshSessionId = event.payload;
    const sessions = useState<Session[]>('sessions', () => []);
    const idx = sessions.value.findIndex((s) => s.sshSessionId === sshSessionId);
    if (idx !== -1) {
      sessions.value[idx] = { ...sessions.value[idx]!, state: 'disconnected' };
    }
  });
}

export const useSessions = createSharedComposable(_useSessions);
