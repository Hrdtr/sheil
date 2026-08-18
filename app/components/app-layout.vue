<script setup lang="ts">
import type { UnlistenFn } from '@tauri-apps/api/event';
import { ArrowLeftRightIcon, FolderIcon, NetworkIcon } from '@lucide/vue';
import { getCurrentWindow } from '@tauri-apps/api/window';
import { platform } from '@tauri-apps/plugin-os';
import { breakpointsTailwind } from '@vueuse/core';

const props = withDefaults(defineProps<{ sidebarOpen?: boolean }>(), {
  sidebarOpen: true,
});
const emit = defineEmits<{
  'update:sidebar-open': [value: boolean];
}>();

const { sessions, activeTabId, switchTab, reorder } = useSessions();
const { settingsTabId } = useSettingsTab();
const { panelOpen: pfOpen, forwards } = usePortForwarding();
const { panelOpen: sftpOpen } = useSftp();
const { md } = useBreakpoints(breakpointsTailwind);

const panelsOpen = computed(() => pfOpen.value || sftpOpen.value);
const isDesktop = computed(() => {
  const p = platform();
  return p !== 'android' && p !== 'ios';
});

watch(md, (value, oldValue) => {
  if (!value && oldValue) {
    emit('update:sidebar-open', false);
  } else if (value && !oldValue) {
    emit('update:sidebar-open', true);
  }
});

const requestDisconnectTab = inject<(tabId: string) => void>('requestDisconnectTab');

const isMacos = platform() === 'macos';
const isFullscreen = ref(false);

const unlistedWindowResizedEvent = ref<UnlistenFn>();
onMounted(async () => {
  const appWindow = getCurrentWindow();
  unlistedWindowResizedEvent.value = await getCurrentWindow().onResized(async () => {
    isFullscreen.value = await appWindow.isFullscreen();
  });
});
onBeforeUnmount(() => {
  unlistedWindowResizedEvent.value?.();
});
</script>

<template>
  <SidebarProvider
    data-tauri-drag-region
    class="bg-sidebar text-sidebar-foreground"
    :open="sidebarOpen"
    @update:open="emit('update:sidebar-open', $event)"
  >
    <AppSidebar class="**:data-[slot='sidebar-container']:ease-linear!" />
    <SidebarInset
      class="flex flex-col h-dvh md:h-[calc(100dvh-calc((--spacing(2))*2))] overflow-hidden transition-all duration-200 ease-linear bg-background/65 shadow-none!"
      :class="
        sidebarOpen ? 'md:my-2! md:mr-2!' : 'h-dvh! m-0! md:peer-data-[variant=inset]:rounded-none'
      "
      data-tauri-drag-region
    >
      <header
        class="flex items-center shrink-0 gap-1.5 w-full px-4 overflow-hidden transition-all duration-200 ease-linear h-14 relative"
        data-tauri-drag-region
      >
        <SidebarTrigger
          variant="secondary"
          class="shrink-0 transition-all duration-200 ease-linear"
          :class="!sidebarOpen && isMacos && !isFullscreen ? 'ml-19.5' : ''"
        />
        <!-- Tab bar -->
        <TabBar
          :sessions="sessions"
          :active-tab-id="activeTabId"
          @select-tab="switchTab"
          @close-tab="(tabId) => requestDisconnectTab?.(tabId)"
          @reorder-tab="(fromIndex, toIndex) => reorder(fromIndex, toIndex)"
        />
        <DropdownMenu v-if="sessions.length > 0 && activeTabId !== settingsTabId">
          <DropdownMenuTrigger as-child>
            <Button
              variant="secondary"
              size="icon-sm"
              class="shrink-0"
              :class="panelsOpen ? 'bg-primary text-primary-foreground hover:bg-primary/90' : ''"
            >
              <NetworkIcon class="size-3.5" />
            </Button>
          </DropdownMenuTrigger>
          <DropdownMenuContent align="end" class="w-auto">
            <DropdownMenuCheckboxItem v-if="isDesktop" v-model="pfOpen">
              <ArrowLeftRightIcon class="size-3.5" />
              <span
                >Port Forwarding<span v-if="forwards.length > 0" class="ml-1 text-muted-foreground"
                  >({{ forwards.length }})</span
                ></span
              >
            </DropdownMenuCheckboxItem>
            <DropdownMenuCheckboxItem v-model="sftpOpen">
              <FolderIcon class="size-3.5" />
              <span>SFTP File Browser</span>
            </DropdownMenuCheckboxItem>
          </DropdownMenuContent>
        </DropdownMenu>
      </header>
      <div class="flex-1 flex flex-col overflow-hidden">
        <slot />
      </div>
    </SidebarInset>
  </SidebarProvider>
</template>
