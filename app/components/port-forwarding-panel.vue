<script setup lang="ts">
import { ArrowLeftRightIcon, ArrowRightLeftIcon, GlobeIcon, PlusIcon, XIcon } from '@lucide/vue';
import { platform } from '@tauri-apps/plugin-os';

const isDesktop = computed(() => {
  const currentPlatform = platform();
  return currentPlatform !== 'android' && currentPlatform !== 'ios';
});

const createDialogOpen = ref(false);

const { activeSession } = useSessions();
const { forwards, stop, refresh, panelOpen } = usePortForwarding();

// Refresh when active session changes
watch(
  () => activeSession.value?.sshSessionId,
  (sessionId) => {
    if (sessionId) refresh(sessionId);
    else forwards.value = [];
  },
  { immediate: true },
);

function labelFor(info: (typeof forwards.value)[number]): string {
  return info.label;
}

function typeBadge(kind: (typeof forwards.value)[number]['kind']): {
  icon: Component;
  text: string;
  class: string;
} {
  switch (kind.type) {
    case 'local':
      return { icon: ArrowRightLeftIcon, text: 'Local', class: 'text-blue-500' };
    case 'remote':
      return { icon: ArrowLeftRightIcon, text: 'Remote', class: 'text-green-500' };
    case 'dynamic':
      return { icon: GlobeIcon, text: 'SOCKS5', class: 'text-purple-500' };
  }
}

async function handleStop(forwardId: string) {
  try {
    await stop(forwardId);
  } catch (e) {
    toast.error(String(e));
  }
}
</script>

<template>
  <div
    v-if="
      isDesktop && panelOpen && activeSession?.sshSessionId && activeSession.state === 'connected'
    "
    class="flex flex-col shrink-0 rounded-lg"
  >
    <!-- Header -->
    <div
      class="flex items-center justify-between pl-1.25 pr-0"
      :class="forwards.length > 0 ? 'pb-1' : ''"
    >
      <div class="flex items-center gap-1.5">
        <span class="text-xs text-muted-foreground">Port Forwarding</span>
        <span v-if="forwards.length > 0" class="text-xs text-muted-foreground">
          ({{ forwards.length }})
        </span>
      </div>
      <Button
        variant="ghost"
        size="icon-sm"
        class="size-5.5 text-muted-foreground hover:text-foreground"
        @click="createDialogOpen = true"
      >
        <PlusIcon class="size-3.5" />
      </Button>
    </div>

    <!-- Active tunnels -->
    <div
      v-if="forwards.length > 0"
      class="flex flex-col gap-0.5 pl-1.25 pr-0 max-h-[10vh] overflow-y-auto"
    >
      <div
        v-for="fwd in forwards"
        :key="fwd.id"
        class="flex items-center justify-between gap-2 rounded-md group"
      >
        <div class="flex items-center gap-2 min-w-0 -ml-px">
          <span
            class="inline-flex items-center gap-1 text-xs font-medium shrink-0"
            :class="typeBadge(fwd.kind).class"
          >
            <Component :is="typeBadge(fwd.kind).icon" class="size-2.5" />
            {{ typeBadge(fwd.kind).text }}
          </span>
          <span
            class="text-xs text-muted-foreground group-hover:text-foreground transition-colors truncate"
            >{{ labelFor(fwd) }}</span
          >
        </div>
        <div class="flex items-center gap-1 shrink-0">
          <span class="size-1.5 rounded-full bg-emerald-500" />
          <Button
            variant="ghost"
            size="icon-sm"
            class="size-5.5 text-muted-foreground hover:text-destructive"
            @click="handleStop(fwd.id)"
          >
            <XIcon class="size-3.5" />
          </Button>
        </div>
      </div>
    </div>

    <PortForwardCreateDialog v-model:open="createDialogOpen" />
  </div>
</template>
