/**
 * Settings tab composable.
 *
 * The settings view is a singleton tab (`kind: 'settings'`) managed by
 * {@link useTabs}. Use `settingsActive` to check whether the settings view is
 * currently displayed, and `openSettings` to open or focus it.
 */
function _useSettingsTab() {
  const { tabs, activeTabId, openTab } = useTabs();

  const settingsTabId = 'settings';

  /** Whether the settings tab is open. */
  const settingsOpen = computed(() => tabs.value.some((tab) => tab.id === settingsTabId));

  /** Whether the settings view is currently the active tab. */
  const settingsActive = computed(() => activeTabId.value === settingsTabId);

  /** Open the settings tab, or focus it when already open. */
  const openSettings = () => {
    openTab('settings', { id: settingsTabId });
  };

  return { settingsTabId, settingsOpen, settingsActive, openSettings };
}

export const useSettingsTab = createSharedComposable(_useSettingsTab);
