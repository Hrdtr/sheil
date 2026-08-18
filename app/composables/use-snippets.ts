type Snippet = Awaited<ReturnType<typeof commands.snippet.list>>[number];

/**
 * Variables auto-filled from the active session's host configuration.
 * Any other `{{variable}}` placeholder prompts the user before execution.
 */
const BUILTIN_VARIABLES = ['host', 'hostname', 'username', 'port'] as const;

interface SnippetRunContext {
  snippet: Snippet;
  /** Every placeholder found in the command, in order of appearance. */
  variables: string[];
  /** Built-in values resolved from the active session's host config. */
  resolved: Record<string, string>;
  /** Variables that require user input before execution. */
  unresolved: string[];
}

/**
 * Shared composable providing reactive access to command snippets.
 *
 * Wraps the Tauri snippet commands with automatic list refresh after
 * mutations, and provides search/filtering plus execution in the active
 * terminal session (with `{{variable}}` substitution). Use
 * {@link useSnippets} — the shared singleton — rather than calling this
 * directly.
 */
function _useSnippets() {
  const { hosts } = useHosts();
  const { activeSession } = useSessions();
  const { focus: focusTerminal } = useTerminalFocus();

  const {
    data: snippets,
    status: snippetsState,
    refresh: refreshSnippets,
  } = useAsyncData(() => commands.snippet.list());

  const groupedSnippets = computed(() => {
    const groups = new Map<string, NonNullable<typeof snippets.value>>();
    for (const snippet of snippets.value || []) {
      const key = snippet.group || 'Other';
      if (!groups.has(key)) groups.set(key, []);
      groups.get(key)!.push(snippet);
    }
    return [...groups.entries()].sort(([a], [b]) => {
      if (a === 'Other') return 1;
      if (b === 'Other') return -1;
      return a.localeCompare(b);
    });
  });

  // ── Filtering ────────────────────────────────────────────────────────────

  /**
   * Filter snippets by free-text query (name, command, description, tags)
   * and optional group / scope constraints.
   */
  function filterSnippets(options: {
    query?: string;
    group?: string | null;
    hostId?: string | null;
    hostGroup?: string | null;
  }): Snippet[] {
    const query = options.query?.trim().toLowerCase() ?? '';
    const group = options.group ?? null;
    const hostId = options.hostId ?? null;
    const hostGroup = options.hostGroup ?? null;

    return (snippets.value ?? []).filter((snippet) => {
      if (group && snippet.group !== group) return false;
      if (hostId && snippet.hostId !== hostId) return false;
      if (hostGroup && snippet.hostGroup !== hostGroup) return false;
      if (!query) return true;

      const haystack = [snippet.name, snippet.command, snippet.description ?? '', ...snippet.tags]
        .join(' ')
        .toLowerCase();
      return haystack.includes(query);
    });
  }

  // ── CRUD ─────────────────────────────────────────────────────────────────

  const create: typeof commands.snippet.create = async (input) => {
    const created = await commands.snippet.create(input);
    await refreshSnippets();
    return created;
  };

  const update: typeof commands.snippet.update = async (id, update) => {
    const updated = await commands.snippet.update(id, update);
    await refreshSnippets();
    return updated;
  };

  const remove = async (id: string) => {
    await commands.snippet.delete(id);
    await refreshSnippets();
  };

  /** Host config of the active session (`null` for quick-connect sessions). */
  const activeHost = computed(() => {
    const hostId = activeSession.value?.hostId;
    if (!hostId) return null;
    return (hosts.value ?? []).find((host) => host.id === hostId) ?? null;
  });

  /**
   * Snippets runnable in the current session: global snippets plus those
   * scoped to the active host or its host group.
   */
  const runnableSnippets = computed(() => {
    const host = activeHost.value;
    return (snippets.value ?? []).filter((snippet) => {
      if (!snippet.hostId && !snippet.hostGroup) return true;
      if (!host) return false;
      if (snippet.hostId) return snippet.hostId === host.id;
      return snippet.hostGroup === host.group;
    });
  });

  // ── Variables ────────────────────────────────────────────────────────────

  /** Extract unique `{{variable}}` placeholder names from a command. */
  function extractVariables(command: string): string[] {
    const names: string[] = [];
    for (const match of command.matchAll(/\{\{\s*([a-zA-Z0-9_-]+)\s*\}\}/g)) {
      const name = match[1]!;
      if (!names.includes(name)) names.push(name);
    }
    return names;
  }

  /** Built-in variable values for a host config. */
  function builtinValues(
    host: {
      name: string;
      host: string;
      username: string;
      port: number;
    } | null,
  ): Record<string, string> {
    if (!host) return {};
    return {
      host: host.host,
      hostname: host.name,
      username: host.username,
      port: String(host.port),
    };
  }

  /** Substitute `{{variable}}` placeholders using the provided values. */
  function applyVariables(command: string, values: Record<string, string>): string {
    return command.replace(/\{\{\s*([a-zA-Z0-9_-]+)\s*\}\}/g, (raw, name: string) => {
      return Object.prototype.hasOwnProperty.call(values, name) ? values[name]! : raw;
    });
  }

  /**
   * Prepare execution of a snippet: extract placeholders and resolve the
   * built-in ones from the active session's host config.
   */
  function prepareRun(snippet: Snippet): SnippetRunContext {
    const variables = extractVariables(snippet.command);
    const resolved = builtinValues(activeHost.value);
    const unresolved = variables.filter(
      (name) => !Object.prototype.hasOwnProperty.call(resolved, name),
    );
    return { snippet, variables, resolved, unresolved };
  }

  // ── Execution ────────────────────────────────────────────────────────────

  /**
   * Execute a snippet in the given SSH session: substitute variables and
   * write the command to the terminal.
   *
   * @param values - User-supplied values for non-built-in variables.
   * @param submit - Append a carriage return so the command runs immediately
   *   (`true` by default). Pass `false` to only insert at the cursor.
   * @returns The SSH session id the command was written to, so callers can
   *   re-focus the terminal after any dialog close animation settles.
   */
  async function run(
    snippet: Snippet,
    values: Record<string, string> = {},
    submit = true,
  ): Promise<string> {
    const session = activeSession.value;
    if (!session?.sshSessionId || session.state !== 'connected') {
      throw new Error('No active terminal session');
    }

    const context = prepareRun(snippet);
    const merged = { ...context.resolved, ...values };
    const command = applyVariables(snippet.command, merged);

    const remaining = extractVariables(command);
    if (remaining.length > 0) {
      throw new Error(`Unresolved variables: ${remaining.map((name) => `{{${name}}}`).join(', ')}`);
    }

    const encoder = new TextEncoder();
    await commands.ssh.write(
      session.sshSessionId,
      encoder.encode(submit ? `${command}\r` : command),
    );
    nextTick(() => focusTerminal(session.sshSessionId));
    return session.sshSessionId;
  }

  return {
    snippets,
    snippetsState,
    groupedSnippets,
    runnableSnippets,
    activeHost,
    refreshSnippets,
    filterSnippets,
    create,
    update,
    remove,
    extractVariables,
    applyVariables,
    builtinValues,
    prepareRun,
    run,
    BUILTIN_VARIABLES,
  };
}

export const useSnippets = createSharedComposable(_useSnippets);
