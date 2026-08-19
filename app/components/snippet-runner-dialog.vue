<script setup lang="ts">
import { FolderIcon, GlobeIcon, MonitorIcon } from '@lucide/vue';

type Snippet = (typeof runnableSnippets.value)[number];

const open = defineModel<boolean>('open');

const { hosts } = useHosts();
const { runnableSnippets } = useSnippets();

const snippetRunDialogRef = useTemplateRef('snippetRunDialog');
const commandInputRef = useTemplateRef('commandInput');

const commandInputSearch = ref('');
const commandInputSearchInputEventListenerHandler = (event: InputEvent) => {
  commandInputSearch.value = (event.target as HTMLInputElement).value;
};

watch(
  commandInputRef,
  (value) => {
    const inputEl = value?.$el.querySelector('input') as HTMLInputElement | undefined;
    if (inputEl) {
      inputEl.addEventListener('input', commandInputSearchInputEventListenerHandler);
    }
  },
  { immediate: true },
);

onBeforeUnmount(() => {
  const inputEl = commandInputRef.value?.$el.querySelector('input') as HTMLInputElement | undefined;
  if (inputEl) {
    inputEl.removeEventListener('input', commandInputSearchInputEventListenerHandler);
  }
});

const groupedSnippets = computed(() => {
  const groups = new Map<string, Snippet[]>();
  for (const snippet of runnableSnippets.value) {
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

const showEmptyStateHelp = computed(() => {
  return commandInputSearch.value.trim() === '' && runnableSnippets.value.length === 0;
});

function scopeInfo(snippet: Snippet): { icon: typeof MonitorIcon; label: string } | null {
  if (snippet.hostId) {
    const host = (hosts.value ?? []).find((host) => host.id === snippet.hostId);
    return { icon: MonitorIcon, label: host?.name ?? 'Unknown host' };
  }
  if (snippet.hostGroup) {
    return { icon: FolderIcon, label: snippet.hostGroup };
  }
  return null;
}

function handleSnippetSelect(snippet: Snippet) {
  open.value = false;
  snippetRunDialogRef.value?.openFor(snippet, true);
}
</script>

<template>
  <CommandDialog
    v-model:open="open"
    title="Run Snippet"
    description="Search your snippets and run them in the active terminal."
    class="**:data-[slot='command-input-wrapper']:pb-2"
  >
    <CommandInput ref="commandInput" placeholder="Search snippets…" />
    <CommandList>
      <div v-if="showEmptyStateHelp" class="p-4 pt-2">
        <p class="text-sm">No snippets available for this session</p>
        <p class="mt-1 text-xs text-muted-foreground">
          Add snippets from the sidebar to run saved commands in one click.
        </p>
      </div>

      <CommandEmpty>No results.</CommandEmpty>

      <CommandGroup
        v-for="[groupName, groupSnippets] in groupedSnippets"
        :key="groupName"
        :heading="groupName"
        class="**:data-[slot='command-group-heading']:text-muted-foreground **:data-[slot='command-group-heading']:px-2 **:data-[slot='command-group-heading']:py-1.5 **:data-[slot='command-group-heading']:text-xs **:data-[slot='command-group-heading']:font-medium"
      >
        <CommandItem
          v-for="snippet in groupSnippets"
          :key="snippet.id"
          :value="snippet.id"
          @select="handleSnippetSelect(snippet)"
        >
          <div class="flex flex-col gap-0.5 min-w-0">
            <span class="flex items-center gap-1.5">
              <span class="truncate text-sm">{{ snippet.name }}</span>
              <span
                v-if="scopeInfo(snippet)"
                class="flex shrink-0 items-center gap-1 text-[11px] text-muted-foreground"
              >
                <component :is="scopeInfo(snippet)!.icon" class="size-3" />
                {{ scopeInfo(snippet)!.label }}
              </span>
              <GlobeIcon v-else class="size-3 shrink-0 text-muted-foreground" />
            </span>
            <span class="truncate font-mono text-xs text-muted-foreground">
              {{ snippet.command }}
            </span>
          </div>
        </CommandItem>
      </CommandGroup>
    </CommandList>
  </CommandDialog>

  <SnippetRunDialog ref="snippetRunDialog" />
</template>

<style>
[data-slot='command-item'][data-highlighted] {
  background-color: var(--muted) !important;
  color: var(--foreground) !important;
}
[data-slot='command-item'][data-highlighted] svg:not([class*='text-']) {
  color: var(--foreground) !important;
}
</style>
