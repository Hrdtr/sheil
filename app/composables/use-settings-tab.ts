function _useSettingsTab() {
  const { sessions, activeTabId } = useSessions();

  const settingsTabId = '__settings';

  function openSettings() {
    if (sessions.value.find((session) => session.tabId === settingsTabId)) {
      activeTabId.value = settingsTabId;
      return;
    }

    sessions.value.push({
      tabId: settingsTabId,
      hostId: '',
      hostName: 'Settings',
      title: null,
      sshSessionId: null,
      state: 'connected' as const,
      error: null,
    });
    activeTabId.value = settingsTabId;
  }

  return { settingsTabId, openSettings };
}

export const useSettingsTab = createSharedComposable(_useSettingsTab);
