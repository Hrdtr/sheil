import type { Terminal } from '@xterm/xterm';

/**
 * Shared registry of live xterm instances keyed by SSH session id.
 *
 * `Terminal` components register on connect and unregister on teardown,
 * letting decoupled UI (e.g. the AI command palette) focus the active
 * terminal without holding a direct reference to it.
 */
function _useTerminalFocus() {
  const terminals = new Map<string, Terminal>();

  function register(sessionId: string, terminal: Terminal): void {
    terminals.set(sessionId, terminal);
  }

  function unregister(sessionId: string): void {
    terminals.delete(sessionId);
  }

  function focus(sessionId: string | null | undefined): void {
    if (!sessionId) return;
    terminals.get(sessionId)?.focus();
  }

  return { register, unregister, focus };
}

export const useTerminalFocus = createSharedComposable(_useTerminalFocus);
