<script setup lang="ts">
import { XIcon } from '@lucide/vue';

type Session = ReturnType<typeof useSessions>['sessions']['value'][number];

const props = defineProps<{
  sessions: Session[];
  activeTabId: string | null;
}>();

const emit = defineEmits<{
  selectTab: [tabId: string];
  closeTab: [tabId: string];
  reorderTab: [fromIndex: number, toIndex: number];
}>();

const { hosts } = useHosts();

function hostName(session: Session): string {
  return (
    session.hostName || hosts.value?.find((h) => h.id === session.hostId)?.name || session.hostId
  );
}

const tabContainerRef = useTemplateRef('tabContainer');

watch(
  () => props.activeTabId,
  (tabId) => {
    setTimeout(() => {
      if (!tabId || !tabContainerRef.value) return;
      const el = tabContainerRef.value.querySelector(`[data-tab-id="${tabId}"]`);
      el?.scrollIntoView({ behavior: 'smooth', block: 'nearest', inline: 'nearest' });
    }, 200);
  },
);

const dragIndex = ref<number | null>(null);
const dropTargetIndex = ref<number | null>(null);
const dragStartX = ref(0);
const dragMoved = ref(false);

function getTabElements(): HTMLElement[] {
  if (!tabContainerRef.value) return [];
  return Array.from(tabContainerRef.value.querySelectorAll('[data-tab-id]'));
}

function findTabIndexFromEvent(event: PointerEvent): number | null {
  const tabs = getTabElements();
  for (let i = 0; i < tabs.length; i++) {
    const rect = tabs[i]!.getBoundingClientRect();
    if (event.clientX >= rect.left && event.clientX <= rect.right) {
      return i;
    }
  }
  return null;
}

function onTabPointerDown(event: PointerEvent, index: number) {
  dragIndex.value = index;
  dragStartX.value = event.clientX;
  dragMoved.value = false;
  window.addEventListener('pointermove', onWindowPointerMove);
  window.addEventListener('pointerup', onWindowPointerUp);
}

function onWindowPointerMove(event: PointerEvent) {
  if (dragIndex.value == null) return;
  if (!dragMoved.value && Math.abs(event.clientX - dragStartX.value) < 5) return;
  dragMoved.value = true;

  const targetIndex = findTabIndexFromEvent(event);
  dropTargetIndex.value =
    targetIndex !== null && targetIndex !== dragIndex.value ? targetIndex : null;
}

function onWindowPointerUp(event: PointerEvent) {
  window.removeEventListener('pointermove', onWindowPointerMove);
  window.removeEventListener('pointerup', onWindowPointerUp);

  if (!dragMoved.value || dragIndex.value == null) {
    dragIndex.value = null;
    dropTargetIndex.value = null;
    return;
  }

  const targetIndex = findTabIndexFromEvent(event);
  if (targetIndex != null && targetIndex !== dragIndex.value) {
    emit('reorderTab', dragIndex.value, targetIndex);
  }

  dragIndex.value = null;
  dropTargetIndex.value = null;
  dragMoved.value = false;
}

if (import.meta.client) {
  onUnmounted(() => {
    window.removeEventListener('pointermove', onWindowPointerMove);
    window.removeEventListener('pointerup', onWindowPointerUp);
  });
}
</script>

<template>
  <div
    v-if="sessions.length > 0"
    ref="tabContainer"
    class="flex-1 flex items-end gap-1 overflow-x-auto shrink-0 scrollbar-none [&::-webkit-scrollbar]:hidden"
    data-tauri-drag-region
  >
    <Button
      v-for="(session, index) in sessions"
      :key="session.tabId"
      :data-tab-id="session.tabId"
      size="sm"
      variant="secondary"
      class="group relative min-w-0 max-w-48 font-normal rounded-lg cursor-default touch-none"
      :class="[
        session.tabId === activeTabId
          ? 'bg-primary hover:bg-primary! text-primary-foreground hover:text-primary-foreground!'
          : 'bg-accent/50 text-foreground/65 hover:text-foreground',
        session.state === 'connecting' && 'animate-pulse',
        session.state === 'error' && 'text-destructive! hover:text-destructive!',
        session.state === 'disconnected' && 'opacity-50!',
        dragIndex === index && 'opacity-40',
      ]"
      @click="emit('selectTab', session.tabId)"
      @pointerdown="(e: PointerEvent) => onTabPointerDown(e, index)"
    >
      <span
        v-if="dropTargetIndex === index && dragIndex! > index"
        class="absolute -left-1 top-0 bottom-0 w-0.5 bg-primary my-1.5 rounded-full z-10"
      />
      <span
        v-if="dropTargetIndex === index && dragIndex! < index"
        class="absolute -right-1 top-0 bottom-0 w-0.5 bg-primary my-1.5 rounded-full z-10"
      />
      <span class="truncate text-sm pointer-events-none">{{
        session.title || hostName(session)
      }}</span>
      <Button
        variant="ghost"
        size="icon-sm"
        class="-mr-1.5 size-6 shrink-0 rounded-md opacity-65 group-hover:opacity-100 transition-opacity"
        :class="
          session.tabId === activeTabId
            ? 'text-primary-foreground group-hover:text-primary-foreground hover:text-primary-foreground hover:bg-accent/10!'
            : 'text-muted-foreground hover:text-muted-foreground hover:bg-accent/50!'
        "
        tabindex="-1"
        @click.stop="emit('closeTab', session.tabId)"
      >
        <XIcon />
      </Button>
    </Button>
  </div>
</template>
