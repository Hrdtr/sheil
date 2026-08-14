import type { Terminal } from '@xterm/xterm';

interface TerminalEntry {
  terminal: Terminal;
  clear: () => void;
  openSearch: () => void;
}

/**
 * Shared registry of live xterm instances keyed by SSH session id.
 *
 * `Terminal` components register on connect and unregister on teardown,
 * letting decoupled UI (e.g. the AI command palette) focus the active
 * terminal without holding a direct reference to it.
 */
function _useTerminalFocus() {
  const terminals = new Map<string, TerminalEntry>();

  function register(sessionId: string, entry: TerminalEntry): void {
    terminals.set(sessionId, entry);
  }

  function unregister(sessionId: string): void {
    terminals.delete(sessionId);
  }

  function focus(sessionId: string | null | undefined): void {
    if (!sessionId) return;
    terminals.get(sessionId)?.terminal.focus();
  }

  function clear(sessionId: string | null | undefined): void {
    if (!sessionId) return;
    terminals.get(sessionId)?.clear();
  }

  function openSearch(sessionId: string | null | undefined): void {
    if (!sessionId) return;
    terminals.get(sessionId)?.openSearch();
  }

  return { register, unregister, focus, clear, openSearch };
}

export const useTerminalFocus = createSharedComposable(_useTerminalFocus);
