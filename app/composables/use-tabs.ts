/** What a tab displays — a live SSH session or a built-in view. */
export type TabKind = 'session' | 'settings';

/** A tab in the tab bar. Session tabs are backed by an SSH session in `useSessions`. */
interface Tab {
  /** Unique tab identifier (also the key linking a session tab to its SSH session). */
  id: string;
  /** What this tab renders. */
  kind: TabKind;
}

/**
 * Tab management composable.
 *
 * Owns the tab bar state only: the ordered list of open tabs, which tab is
 * active, and how tabs are opened, closed, focused, and reordered. It knows
 * nothing about SSH — session tabs get their connection state from
 * {@link useSessions}, which keys sessions by tab id.
 *
 * Wrapped with {@link createSharedComposable} so the function body runs once —
 * `tabCounter` and {@link useState} keys are shared across components.
 *
 * @returns
 * - `tabs` — reactive array of open tabs (display order)
 * - `activeTabId` — id of the focused tab (`null` when none open)
 * - `activeTab` — the currently focused tab (or `null`)
 * - `openTab(kind, options)` — open (or focus) a tab and activate it
 * - `closeTab(id)` — remove a tab; focus shifts to the last remaining tab
 * - `switchTab(id)` — activate a tab by id
 * - `reorderTabs(fromIndex, toIndex)` — move a tab within the tab bar
 */
function _useTabs() {
  let tabCounter = 0;
  function newTabId(): string {
    tabCounter += 1;
    return `tab_${tabCounter}`;
  }

  // ---- State ----

  const tabs = useState<Tab[]>('tabs', () => []);
  const activeTabId = useState<string | null>('tabs:active-tab-id', () => null);

  const activeTab = computed<Tab | null>(() => {
    return tabs.value.find((tab) => tab.id === activeTabId.value) ?? null;
  });

  // ---- Public API ----

  /**
   * Open a new tab and activate it.
   *
   * Passing an explicit `id` creates a singleton tab: if a tab with that id
   * already exists it is only focused. `insertIndex` controls where the tab
   * is inserted (defaults to the end) — used to reconnect in place.
   *
   * @returns The id of the opened (or focused) tab.
   */
  const openTab = (kind: TabKind, options: { id?: string; insertIndex?: number } = {}): string => {
    const id = options.id ?? newTabId();

    const existing = tabs.value.find((tab) => tab.id === id);
    if (existing) {
      activeTabId.value = id;
      return id;
    }

    const tab: Tab = { id, kind };
    const { insertIndex } = options;
    if (insertIndex !== undefined && insertIndex >= 0 && insertIndex <= tabs.value.length) {
      tabs.value.splice(insertIndex, 0, tab);
    } else {
      tabs.value.push(tab);
    }
    activeTabId.value = id;
    return id;
  };

  /**
   * Remove a tab. When the removed tab was the active one, focus shifts to
   * the last remaining tab (or `null` if none remain).
   */
  const closeTab = (id: string): void => {
    tabs.value = tabs.value.filter((tab) => tab.id !== id);
    if (activeTabId.value === id) {
      activeTabId.value = tabs.value.length > 0 ? tabs.value[tabs.value.length - 1]!.id : null;
    }
  };

  /** Activate a tab by id. No-op if the id doesn't match an existing tab. */
  const switchTab = (id: string): void => {
    if (tabs.value.some((tab) => tab.id === id)) {
      activeTabId.value = id;
    }
  };

  /** Move a tab from `fromIndex` to `toIndex` within the tab bar. */
  const reorderTabs = (fromIndex: number, toIndex: number): void => {
    const list = [...tabs.value];
    const [item] = list.splice(fromIndex, 1);
    list.splice(toIndex, 0, item!);
    tabs.value = list;
  };

  return { tabs, activeTabId, activeTab, openTab, closeTab, switchTab, reorderTabs };
}

export const useTabs = createSharedComposable(_useTabs);
