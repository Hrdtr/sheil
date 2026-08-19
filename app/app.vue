<script setup lang="ts">
import { getCurrentWindow } from '@tauri-apps/api/window';
import { Toaster } from '@/components/ui/sonner';
import 'vue-sonner/style.css';

const { tabs, activeTabId, switchTab, closeTab } = useTabs();
const { sessions, activeSession, connect, focusOrConnect, disconnect, setTitle } = useSessions();
const { recentHosts, clear: clearRecent } = useRecentHosts();
const {
  confirmCloseEnabled,
  confirmCloseDialogOpen,
  confirmClose,
  cancelClose,
  showConfirmCloseDialog,
} = useConfirmClose();
const { openSettings } = useSettingsTab();
const { togglePanel: togglePortForwardingPanel } = usePortForwarding();
const { togglePanel: toggleSftpPanel } = useSftp();
const { enabled: aiEnabled, commandGeneratorEnabled: aiCommandGeneratorEnabled } = useAiSettings();
const { checkForUpdates } = useUpdater();
const { clear: clearTerminal, openSearch: openTerminalSearch } = useTerminalFocus();
const { fontSize: terminalFontSize, defaultAppearance: terminalDefaultAppearance } =
  useTerminalSettings();

onMounted(() => {
  if (!import.meta.dev) {
    setTimeout(() => checkForUpdates(), 5_000);
  }
});

const activeSessionCount = computed(
  () =>
    sessions.value.filter(
      (session) => session.state === 'connected' || session.state === 'connecting',
    ).length,
);

/** Tabs joined with their live session (null for non-session tabs like Settings). */
const tabSessions = computed(() =>
  tabs.value.map((tab) => ({
    tab,
    session: sessions.value.find((session) => session.tabId === tab.id) ?? null,
  })),
);

const sidebarOpen = ref<boolean>();
const quickConnectOpen = ref(false);
const aiCommandGeneratorOpen = ref(false);
const snippetRunnerOpen = ref(false);

const disconnectConfirmOpen = ref(false);
const pendingDisconnectTabId = ref<string | null>(null);
const pendingDisconnectHostName = computed(() => {
  const tabId = pendingDisconnectTabId.value;
  if (!tabId) return '';
  return sessions.value.find((session) => session.tabId === tabId)?.hostName ?? '';
});

function requestDisconnectTab(tabId: string) {
  const session = sessions.value.find((session) => session.tabId === tabId);
  if (!session) {
    closeTab(tabId);
    return;
  }
  if (session.state !== 'connected' && session.state !== 'connecting') {
    disconnect(session.tabId);
    return;
  }
  pendingDisconnectTabId.value = session.tabId;
  disconnectConfirmOpen.value = true;
}

function confirmDisconnectTab() {
  const tabId = pendingDisconnectTabId.value;
  disconnectConfirmOpen.value = false;
  pendingDisconnectTabId.value = null;
  if (tabId) disconnect(tabId);
}

function cancelDisconnectTab() {
  disconnectConfirmOpen.value = false;
  pendingDisconnectTabId.value = null;
}

provide('requestDisconnectTab', requestDisconnectTab);
provide('openQuickConnectDialog', () => (quickConnectOpen.value = true));

useEventListener('contextmenu', (e) => {
  const target = e.target as HTMLElement;
  if (
    target.tagName === 'INPUT' ||
    target.tagName === 'TEXTAREA' ||
    target.isContentEditable ||
    target.closest('.allow-select')
  ) {
    return;
  }
  e.preventDefault();
});

defineShortcuts({
  'meta_shift_arrowleft': {
    handler: () => {
      const list = tabs.value;
      if (list.length < 2) return;
      const idx = list.findIndex((tab) => tab.id === activeTabId.value);
      const prev = idx <= 0 ? list[list.length - 1]! : list[idx - 1]!;
      switchTab(prev.id);
    },
    usingInput: false,
  },
  'meta_shift_arrowright': {
    handler: () => {
      const list = tabs.value;
      if (list.length < 2) return;
      const idx = list.findIndex((tab) => tab.id === activeTabId.value);
      const next = idx >= list.length - 1 ? list[0]! : list[idx + 1]!;
      switchTab(next.id);
    },
    usingInput: false,
  },
  'meta_1': {
    handler: () => switchTab(tabs.value[0]?.id ?? ''),
    usingInput: true,
  },
  'meta_2': {
    handler: () => switchTab(tabs.value[1]?.id ?? ''),
    usingInput: true,
  },
  'meta_3': {
    handler: () => switchTab(tabs.value[2]?.id ?? ''),
    usingInput: true,
  },
  'meta_4': {
    handler: () => switchTab(tabs.value[3]?.id ?? ''),
    usingInput: true,
  },
  'meta_5': {
    handler: () => switchTab(tabs.value[4]?.id ?? ''),
    usingInput: true,
  },
  'meta_6': {
    handler: () => switchTab(tabs.value[5]?.id ?? ''),
    usingInput: true,
  },
  'meta_7': {
    handler: () => switchTab(tabs.value[6]?.id ?? ''),
    usingInput: true,
  },
  'meta_8': {
    handler: () => switchTab(tabs.value[7]?.id ?? ''),
    usingInput: true,
  },
  'meta_9': () => {
    const list = tabs.value;
    if (list.length > 0) switchTab(list[list.length - 1]!.id);
  },
  'meta_0': {
    handler: () => {
      terminalFontSize.value = terminalDefaultAppearance.fontSize;
    },
    usingInput: true,
  },
  'meta_=': {
    handler: () => {
      terminalFontSize.value = terminalFontSize.value + 1;
    },
    usingInput: true,
  },
  'meta_-': {
    handler: () => {
      terminalFontSize.value = terminalFontSize.value - 1;
    },
    usingInput: true,
  },
  'meta_w': {
    handler: () => {
      if (!activeTabId.value) return;
      requestDisconnectTab(activeTabId.value);
    },
    usingInput: true,
  },
  'meta_t': {
    handler: () => {
      quickConnectOpen.value = !quickConnectOpen.value;
    },
    usingInput: true,
  },
  'meta_shift_f': {
    handler: () => {
      togglePortForwardingPanel();
    },
    usingInput: true,
  },
  'meta_shift_s': {
    handler: () => {
      toggleSftpPanel();
    },
    usingInput: true,
  },
  'meta_i': {
    handler: () => {
      if (!aiEnabled.value || !aiCommandGeneratorEnabled.value) return;
      if (activeSession.value?.state !== 'connected') return;
      aiCommandGeneratorOpen.value = true;
    },
    usingInput: true,
  },
  'meta_shift_p': {
    handler: () => {
      if (activeSession.value?.state !== 'connected') return;
      snippetRunnerOpen.value = true;
    },
    usingInput: true,
  },
  'meta_k': {
    handler: () => clearTerminal(activeSession.value?.sshSessionId),
    usingInput: true,
  },
  'meta_f': {
    handler: () => openTerminalSearch(activeSession.value?.sshSessionId),
    usingInput: true,
  },
  'meta_,': {
    handler: () => openSettings(),
    usingInput: false,
  },
  'meta_q': {
    handler: () => {
      if (confirmCloseEnabled.value && activeSessionCount.value > 0) {
        showConfirmCloseDialog({ destroy: true });
      } else {
        getCurrentWindow().destroy();
      }
    },
    usingInput: false,
  },
});

async function reconnect() {
  const tabId = activeTabId.value;
  const hostId = activeSession.value?.hostId;
  if (!tabId || !hostId) return;
  const index = tabs.value.findIndex((tab) => tab.id === tabId);
  await disconnect(tabId);
  await connect(hostId, index);
}
</script>

<template>
  <AppLayout v-model:sidebar-open="sidebarOpen">
    <div class="flex-1 flex flex-col w-full h-full overflow-hidden">
      <template v-for="{ tab, session } in tabSessions" :key="tab.id">
        <div
          v-show="tab.id === activeTabId"
          class="flex-1 flex flex-col w-full h-full overflow-hidden"
        >
          <SettingsView v-if="tab.kind === 'settings'" />

          <div v-else class="flex-1 flex flex-col w-full h-full overflow-hidden">
            <div
              v-if="session?.sshSessionId && session.state === 'connected'"
              class="flex-1 w-full px-4 pb-4"
            >
              <Terminal
                :session-id="session.sshSessionId"
                :on-title-change="(title: string) => setTitle(tab.id, title)"
              />
            </div>
            <div
              v-else-if="session?.state === 'connecting'"
              class="flex items-center justify-center h-full text-sm text-muted-foreground pb-14"
            >
              Connecting to {{ session.hostName }}…
            </div>
            <div
              v-else-if="session?.state === 'error'"
              class="flex items-center justify-center h-full text-sm text-destructive pb-14"
            >
              Error: {{ session.error }}
            </div>
            <div
              v-else-if="session?.state === 'disconnected'"
              class="flex flex-col items-center justify-center h-full gap-3 text-sm text-muted-foreground pb-14"
            >
              <span>Connection to {{ session.hostName }} closed.</span>
              <Button variant="outline" size="sm" @click="reconnect"> Reconnect </Button>
            </div>

            <div class="flex flex-col gap-3">
              <SftpPanel class="mx-4 last:mb-4 shrink-0" />
              <PortForwardingPanel class="mx-4 last:mb-4 shrink-0" />
            </div>
          </div>
        </div>
      </template>
      <div
        v-if="tabs.length === 0"
        class="grid place-items-center w-full h-full overflow-y-auto select-none"
      >
        <div class="flex flex-col gap-10 w-full max-w-md mx-auto px-6 pb-14">
          <div class="flex flex-col gap-1">
            <span class="text-base font-semibold">Welcome to Sheil</span>
            <span class="text-xs text-muted-foreground leading-normal"
              >Add, manage, and connect to your hosts, or quick connect from the sidebar.</span
            >
          </div>
          <div v-if="recentHosts.length > 0" class="w-full">
            <div class="flex items-center justify-between mb-2">
              <span class="text-xs font-medium text-muted-foreground tracking-wide"
                >Recent Connections</span
              >
              <Button
                variant="ghost"
                size="sm"
                class="text-xs h-auto px-0 py-0 text-foreground/75 hover:text-foreground hover:bg-transparent"
                @click="clearRecent"
                >Clear</Button
              >
            </div>
            <div class="flex flex-col">
              <Button
                v-for="host in recentHosts"
                :key="host.id"
                variant="ghost"
                class="justify-start font-normal h-auto py-2 px-3 -mx-3 rounded-lg"
                @click="focusOrConnect(host.id)"
                @dblclick="connect(host.id)"
              >
                <div class="flex flex-col min-w-0 text-left">
                  <span class="text-sm truncate">{{ host.name }}</span>
                  <span class="text-xs text-muted-foreground truncate"
                    >{{ host.username }}@{{ host.host }}</span
                  >
                </div>
              </Button>
            </div>
          </div>
        </div>
      </div>
    </div>
  </AppLayout>

  <Toaster />

  <AlertDialog :open="confirmCloseDialogOpen">
    <AlertDialogContent>
      <AlertDialogHeader>
        <AlertDialogTitle>Quit Sheil?</AlertDialogTitle>
        <AlertDialogDescription>
          {{ activeSessionCount }} connection(s) are still open. Quitting will disconnect them.
        </AlertDialogDescription>
      </AlertDialogHeader>
      <AlertDialogFooter>
        <AlertDialogCancel variant="outline" @click="cancelClose">Cancel</AlertDialogCancel>
        <AlertDialogAction variant="destructive" @click="confirmClose"
          >Disconnect &amp; Quit</AlertDialogAction
        >
      </AlertDialogFooter>
    </AlertDialogContent>
  </AlertDialog>

  <AlertDialog :open="disconnectConfirmOpen">
    <AlertDialogContent>
      <AlertDialogHeader>
        <AlertDialogTitle>Disconnect {{ pendingDisconnectHostName }}?</AlertDialogTitle>
        <AlertDialogDescription>
          Your active session will be closed and any running processes on the remote host may be
          interrupted.
        </AlertDialogDescription>
      </AlertDialogHeader>
      <AlertDialogFooter>
        <AlertDialogCancel variant="outline" @click="cancelDisconnectTab">Cancel</AlertDialogCancel>
        <AlertDialogAction variant="destructive" @click="confirmDisconnectTab">
          Disconnect
        </AlertDialogAction>
      </AlertDialogFooter>
    </AlertDialogContent>
  </AlertDialog>

  <QuickConnectDialog v-model:open="quickConnectOpen" />
  <AiCommandGeneratorDialog v-model:open="aiCommandGeneratorOpen" />
  <SnippetRunnerDialog v-model:open="snippetRunnerOpen" />
</template>

<style>
body {
  overflow: hidden;
}

*,
*::before,
*::after {
  user-select: none;
}

input,
textarea,
[contenteditable] {
  user-select: text;
}

.allow-select {
  user-select: text;
}

li[data-sonner-toast] [data-description] {
  color: var(--muted-foreground) !important;
}
</style>
