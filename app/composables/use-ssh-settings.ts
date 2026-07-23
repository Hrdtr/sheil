/**
 * Runtime-configurable SSH connection settings.
 *
 * Wrapped with {@link createSharedComposable} so the function body
 * runs once — all components share the same reactive state.
 */
interface SshSettings {
  /** Seconds between keepalive packets (null = disabled). */
  keepaliveInterval: number | null;
  /** Seconds before giving up on a connection attempt (null = use default). */
  connectTimeout: number | null;
}

function _useSshSettings() {
  const defaultSshSettings: SshSettings = {
    keepaliveInterval: null,
    connectTimeout: null,
  };

  const sshSettings = useLocalStorage('ssh-settings', () => defaultSshSettings);

  const keepaliveIntervalMin = 0;
  const keepaliveIntervalMax = 3600;

  function clampKeepaliveInterval(v: number): number {
    if (Number.isNaN(v)) return 0;
    return Math.min(keepaliveIntervalMax, Math.max(keepaliveIntervalMin, Math.round(v)));
  }

  const keepaliveInterval = computed({
    get: () => sshSettings.value.keepaliveInterval,
    set: (value) => {
      sshSettings.value = {
        ...sshSettings.value,
        keepaliveInterval: value != null ? clampKeepaliveInterval(value) : null,
      };
    },
  });

  const connectTimeoutMin = 1;
  const connectTimeoutMax = 300;

  function clampConnectTimeout(v: number): number {
    if (Number.isNaN(v)) return 30;
    return Math.min(connectTimeoutMax, Math.max(connectTimeoutMin, Math.round(v)));
  }

  const connectTimeout = computed({
    get: () => sshSettings.value.connectTimeout,
    set: (value) => {
      sshSettings.value = {
        ...sshSettings.value,
        connectTimeout: value != null ? clampConnectTimeout(value) : null,
      };
    },
  });

  return {
    keepaliveInterval,
    keepaliveIntervalMin,
    keepaliveIntervalMax,
    connectTimeout,
    connectTimeoutMin,
    connectTimeoutMax,
  };
}

export const useSshSettings = createSharedComposable(_useSshSettings);
