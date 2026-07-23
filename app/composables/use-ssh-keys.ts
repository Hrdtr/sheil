/**
 * Shared composable providing reactive access to imported SSH private keys.
 *
 * Wraps the Tauri SSH key commands with automatic list refresh after
 * mutations (import, delete). Use {@link useSshKeys} — the shared
 * singleton — rather than calling commands directly.
 */
function _useSshKeys() {
  const {
    data: keys,
    status: keysState,
    refresh: refreshKeys,
  } = useAsyncData(() => commands.ssh.listKeys(), { default: () => [] });

  /**
   * Import an SSH private key into encrypted storage.
   * Automatically refreshes the key list on success.
   */
  const importKey: typeof commands.ssh.importKey = async (name, keyData, passphrase?) => {
    const info = await commands.ssh.importKey(name, keyData, passphrase);
    await refreshKeys();
    return info;
  };

  /**
   * Delete an SSH private key from encrypted storage and metadata table.
   * Automatically refreshes the key list on success.
   */
  const deleteKey: typeof commands.ssh.deleteKey = async (name) => {
    await commands.ssh.deleteKey(name);
    await refreshKeys();
  };

  return {
    keys,
    keysState,
    refreshKeys,
    importKey,
    deleteKey,
  };
}

export const useSshKeys = createSharedComposable(_useSshKeys);
