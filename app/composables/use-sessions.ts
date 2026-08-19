import { listen } from '@tauri-apps/api/event';

type SessionState = 'connecting' | 'connected' | 'error' | 'disconnected';

/** A live SSH connection attached to a session tab (see `useTabs`). */
interface Session {
  /** Id of the tab this session is displayed in. */
  tabId: string;
  /** Id of the host configuration this session was launched from ('' for quick connect). */
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
 * SSH session lifecycle composable.
 *
 * Owns the connection state of every open session tab: initiating
 * connections, tracking progress, and disconnecting. Tab chrome (ordering,
 * activation, close/refocus behavior) lives in {@link useTabs} — sessions are
 * keyed by tab id and only exist for `kind: 'session'` tabs, so
 * `activeSession` is `null` whenever a non-session tab (e.g. Settings) is
 * active.
 *
 * Wrapped with {@link createSharedComposable} so the function body
 * runs once — computed refs and {@link useState} keys are shared across
 * components.
 *
 * @returns
 * - `sessions` — reactive array of live sessions
 * - `activeSession` — the session of the currently focused tab (or `null`)
 * - `connect(hostId)` — open a session tab and initiate SSH
 * - `connectDirect(...)` — connect without a saved host config
 * - `disconnect(tabId)` — close the SSH connection and its tab
 * - `focusOrConnect(hostId)` — focus an existing session tab or connect
 * - `setTitle(tabId, title)` — update the dynamic terminal title
 */
function _useSessions() {
  const { keepaliveInterval, connectTimeout } = useSshSettings();
  const { activeTabId, openTab, closeTab, switchTab } = useTabs();

  // ---- State ----

  const sessions = useState<Session[]>('sessions', () => []);

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
   * Resolve a host by id, open a session tab, and initiate an SSH connection.
   *
   * The host is resolved from the database first so the session only stores
   * the `hostId` rather than a potentially stale full config object. The tab
   * and session are created immediately in `'connecting'` state so the UI can
   * show feedback before the connection completes.
   *
   * On success the session transitions to `'connected'`; on failure it
   * transitions to `'error'` and the error is rethrown.
   *
   * @param hostId - Id of the host configuration to connect to.
   * @param insertIndex - Optional tab index to insert at (for reconnection).
   * @returns The backend SSH session id.
   */
  const connect = async (hostId: string, insertIndex?: number): Promise<string> => {
    const hostConfig = await commands.host.resolve(hostId);
    const tabId = openTab('session', { insertIndex });

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
    sessions.value = [...sessions.value, session];

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
   * Connect to a host directly without a saved host config, opening a new
   * session tab. Used by the Quick Connect dialog for on-the-fly connections.
   */
  const connectDirect = async (
    host: string,
    port: number,
    username: string,
    auth: SshAuth,
  ): Promise<string> => {
    const tabId = openTab('session');

    sessions.value = [
      ...sessions.value,
      {
        tabId,
        hostId: '',
        hostName: `${username}@${host}`,
        title: null,
        sshSessionId: null,
        state: 'connecting',
        error: null,
      },
    ];

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

  /**
   * Disconnect an SSH session and remove its tab.
   *
   * The SSH channel is closed on the backend; the PTY is cleaned up by
   * {@link TerminalView}'s `onUnmounted` hook. This is best-effort — if the
   * SSH session is already gone, the disconnect call is skipped.
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
    closeTab(tabId);
  };

  /**
   * Focus an existing session tab for a host, or open a new connection when
   * none exists.
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

  /** Update the dynamic title for a session (called from Terminal onTitleChange). */
  const setTitle = (tabId: string, title: string): void => {
    sessions.value = sessions.value.map((session) =>
      session.tabId === tabId ? { ...session, title } : session,
    );
  };

  return {
    sessions,
    activeSession,
    connect,
    connectDirect,
    disconnect,
    focusOrConnect,
    setTitle,
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
