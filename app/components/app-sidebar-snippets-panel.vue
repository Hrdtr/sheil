<script setup lang="ts">
import {
  CopyIcon,
  EllipsisVerticalIcon,
  EditIcon,
  MonitorIcon,
  PlayIcon,
  TerminalIcon,
  TrashIcon,
  FolderIcon,
} from '@lucide/vue';

type Snippet = NonNullable<ReturnType<typeof useSnippets>['snippets']['value']>[number];

const { hosts } = useHosts();
const { activeSession } = useSessions();
const { groupedSnippets, filterSnippets, remove } = useSnippets();

const snippetFormDialogRef = useTemplateRef('snippetFormDialog');
const snippetRunDialogRef = useTemplateRef('snippetRunDialog');

const query = ref('');
const visibleGroups = computed(() => {
  if (!query.value.trim()) return groupedSnippets.value;
  const filtered = filterSnippets({ query: query.value });
  const groups = new Map<string, Snippet[]>();
  for (const snippet of filtered) {
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

function scopeLabel(snippet: Snippet): string | null {
  if (snippet.hostId) {
    const host = (hosts.value ?? []).find((host) => host.id === snippet.hostId);
    return host?.name ?? 'Unknown host';
  }
  if (snippet.hostGroup) return snippet.hostGroup;
  return null;
}

function runSnippet(snippet: Snippet, submit = true) {
  snippetRunDialogRef.value?.openFor(snippet, submit);
}

async function copySnippet(snippet: Snippet) {
  try {
    await navigator.clipboard.writeText(snippet.command);
    toast.success('Command copied to clipboard');
  } catch {
    toast.error('Failed to copy command');
  }
}

// ── Delete ─────────────────────────────────────────────────────────────────

const deleteId = ref<string | null>(null);
const deleteConfirmDialogOpen = ref(false);

watch(deleteId, (value) => {
  deleteConfirmDialogOpen.value = !!value;
});
watch(deleteConfirmDialogOpen, (value) => {
  if (!value) {
    setTimeout(() => {
      deleteId.value = null;
    }, 300);
  }
});

async function handleDelete() {
  const id = deleteId.value;
  if (!id) return;
  try {
    await remove(id);
    toast.success('Snippet deleted');
    deleteId.value = null;
  } catch (err) {
    toast.error(err instanceof Error ? err.message : String(err));
  }
}

// ── Public API (for the header "+" dropdown) ───────────────────────────────

const templatesDialogOpen = ref(false);

const openAdd = () => snippetFormDialogRef.value?.openAdd();
const openTemplates = () => {
  templatesDialogOpen.value = true;
};

function addFromTemplate(template: {
  name: string;
  command: string;
  description: string;
  group: string;
  tags: string[];
}) {
  snippetFormDialogRef.value?.openAdd(template);
}

defineExpose({ openAdd, openTemplates });
</script>

<template>
  <div data-tauri-drag-region>
    <SidebarGroup class="pt-0 sticky top-0 z-10 bg-sidebar" data-tauri-drag-region>
      <SidebarGroupLabel class="px-4.5 text-sm text-sidebar-foreground" data-tauri-drag-region
        >Snippets</SidebarGroupLabel
      >
      <SidebarGroupContent class="pl-4 pr-2.5" data-tauri-drag-region>
        <Input
          v-model="query"
          class="flex-1 h-7.5 text-sm!"
          placeholder="Search..."
          autocomplete="off"
        />
      </SidebarGroupContent>
    </SidebarGroup>

    <div
      v-if="(query.trim() ? visibleGroups.length : groupedSnippets.length) === 0"
      class="flex flex-col justify-center gap-2 px-4 py-4"
      data-tauri-drag-region
    >
      <div class="space-y-1" data-tauri-drag-region>
        <p class="text-sm font-medium" data-tauri-drag-region>
          {{ query.trim() ? 'No snippets match' : 'No snippets yet' }}
        </p>
        <p class="text-xs text-muted-foreground leading-normal" data-tauri-drag-region>
          {{
            query.trim()
              ? 'Try a different search term.'
              : 'Save frequently-used commands and run them in one click'
          }}
        </p>
      </div>
    </div>

    <div v-else data-tauri-drag-region>
      <SidebarGroup
        v-for="[groupName, groupSnippets] in visibleGroups"
        :key="groupName"
        data-tauri-drag-region
      >
        <SidebarGroupLabel class="px-4.5" data-tauri-drag-region>{{ groupName }}</SidebarGroupLabel>
        <SidebarGroupContent data-tauri-drag-region>
          <SidebarMenu class="gap-0.5" data-tauri-drag-region>
            <SidebarMenuItem
              v-for="snippet in groupSnippets"
              :key="snippet.id"
              class="mx-2 group/item"
              data-tauri-drag-region
            >
              <Tooltip>
                <TooltipTrigger as-child>
                  <SidebarMenuButton
                    size="lg"
                    class="hover:bg-accent dark:hover:bg-accent/50 transition-colors rounded-lg h-fit group/button px-2.5 py-[5.5px] items-start gap-1.5"
                    @click="
                      () => {
                        if (activeSession && activeSession.state === 'connected') {
                          runSnippet(snippet);
                        } else {
                          copySnippet(snippet);
                        }
                      }
                    "
                  >
                    <div
                      class="flex flex-col gap-0.5 min-w-0 text-muted-foreground group-hover/button:text-foreground transition-colors"
                    >
                      <span class="truncate">{{ snippet.name }}</span>
                      <span
                        class="truncate font-mono text-xs text-muted-foreground -mt-4 group-hover/button:mt-0 opacity-0 group-hover/button:opacity-100 transition-all duration-200 ease-in-out"
                      >
                        {{ snippet.command }}
                      </span>
                    </div>
                  </SidebarMenuButton>
                </TooltipTrigger>
                <TooltipContent as="div" class="flex flex-col items-start gap-0.5">
                  <span class="text-xs">{{ snippet.name }}</span>
                  <span class="font-[11px] max-w-64 truncate font-mono text-xs opacity-50">{{
                    snippet.command
                  }}</span>
                  <div
                    v-if="scopeLabel(snippet)"
                    class="flex items-center gap-1 text-xs text-muted-foreground mt-1"
                  >
                    <component :is="snippet.hostId ? MonitorIcon : FolderIcon" class="size-3" />
                    <span class="truncate">{{ scopeLabel(snippet) }}</span>
                  </div>
                </TooltipContent>
              </Tooltip>
              <DropdownMenu>
                <DropdownMenuTrigger as-child>
                  <SidebarMenuAction
                    class="-mt-1 opacity-50 group-hover/item:opacity-100 transition-opacity"
                  >
                    <EllipsisVerticalIcon />
                  </SidebarMenuAction>
                </DropdownMenuTrigger>
                <DropdownMenuContent class="w-fit">
                  <DropdownMenuGroup>
                    <DropdownMenuItem
                      v-if="activeSession && activeSession.state === 'connected'"
                      @click="runSnippet(snippet)"
                    >
                      <PlayIcon class="size-3.5" />
                      <span>Run</span>
                    </DropdownMenuItem>
                    <DropdownMenuItem
                      v-if="activeSession && activeSession.state === 'connected'"
                      @click="runSnippet(snippet, false)"
                    >
                      <TerminalIcon class="size-3.5" />
                      <span>Insert without running</span>
                    </DropdownMenuItem>
                    <DropdownMenuItem @click="copySnippet(snippet)">
                      <CopyIcon class="size-3.5" />
                      <span>Copy command</span>
                    </DropdownMenuItem>
                    <DropdownMenuItem @click="snippetFormDialogRef?.openEdit(snippet.id)">
                      <EditIcon class="size-3.5" />
                      <span>Edit</span>
                    </DropdownMenuItem>
                    <DropdownMenuItem variant="destructive" @click="deleteId = snippet.id">
                      <TrashIcon class="size-3.5" />
                      <span>Delete</span>
                    </DropdownMenuItem>
                  </DropdownMenuGroup>
                </DropdownMenuContent>
              </DropdownMenu>
            </SidebarMenuItem>
          </SidebarMenu>
        </SidebarGroupContent>
      </SidebarGroup>
    </div>

    <SnippetFormDialog ref="snippetFormDialog" />

    <SnippetTemplatesDialog v-model:open="templatesDialogOpen" @select="addFromTemplate" />

    <SnippetRunDialog ref="snippetRunDialog" />

    <AlertDialog v-model:open="deleteConfirmDialogOpen">
      <AlertDialogContent>
        <AlertDialogHeader>
          <AlertDialogTitle>Delete snippet?</AlertDialogTitle>
          <AlertDialogDescription>
            This snippet will be permanently deleted.
          </AlertDialogDescription>
        </AlertDialogHeader>
        <AlertDialogFooter>
          <AlertDialogCancel variant="outline">Cancel</AlertDialogCancel>
          <AlertDialogAction @click="handleDelete" variant="destructive">Delete</AlertDialogAction>
        </AlertDialogFooter>
      </AlertDialogContent>
    </AlertDialog>
  </div>
</template>
