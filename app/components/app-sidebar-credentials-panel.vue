<script setup lang="ts">
import {
  EllipsisVerticalIcon,
  EditIcon,
  KeyIcon,
  LinkIcon,
  LockIcon,
  TrashIcon,
} from '@lucide/vue';

const { hosts } = useHosts();
const { groupedCredentials, filterCredentials, assignToHost, remove } = useCredentials();

const credentialFormDialogRef = useTemplateRef('credentialFormDialog');

// ── Usage counts ────────────────────────────────────────────────────────────

const usageCount = computed(() => {
  const counts = new Map<string, number>();
  for (const host of hosts.value ?? []) {
    if (host.keyId) counts.set(host.keyId, (counts.get(host.keyId) ?? 0) + 1);
    if (host.passwordId) counts.set(host.passwordId, (counts.get(host.passwordId) ?? 0) + 1);
  }
  return counts;
});

const query = ref('');

const visibleGroups = computed(() => {
  if (!query.value.trim()) return groupedCredentials.value;
  const filtered = filterCredentials({ query: query.value });
  const groups = new Map<string, Credential[]>();
  for (const credential of filtered) {
    const key = credential.group || 'Other';
    if (!groups.has(key)) groups.set(key, []);
    groups.get(key)!.push(credential);
  }
  return [...groups.entries()].sort(([a], [b]) => {
    if (a === 'Other') return 1;
    if (b === 'Other') return -1;
    return a.localeCompare(b);
  });
});

type Credential = (typeof groupedCredentials.value)[number][1][number];

function credentialSubtitle(credential: Credential): string {
  const count = usageCount.value.get(credential.id) ?? 0;
  const usage = count > 0 ? `${count} ${count === 1 ? 'host' : 'hosts'} assigned` : null;

  if (credential.kind === 'key') {
    return ['key', credential.keyType, credential.keyFingerprint, usage ?? 'Not used'].join(' · ');
  }
  return `Password · ${usage ?? 'Not used'}`;
}

// ── Assign to host ──────────────────────────────────────────────────────────

const assignCredentialId = ref<string | null>(null);
const assignHostId = ref<string | null>(null);
const assignDialogOpen = ref(false);

const assigningCredential = computed(() => {
  if (!assignCredentialId.value) return null;
  return (
    groupedCredentials.value
      .flatMap(([, list]) => list)
      .find((credential) => credential.id === assignCredentialId.value) ?? null
  );
});

function openAssign(credentialId: string) {
  assignCredentialId.value = credentialId;
  assignHostId.value = null;
  assignDialogOpen.value = true;
}

async function handleAssign() {
  const credential = assigningCredential.value;
  if (!credential || !assignHostId.value) return;
  try {
    await assignToHost(assignHostId.value, credential);
    toast.success(`Assigned "${credential.name}" to host`);
    assignDialogOpen.value = false;
  } catch (err) {
    toast.error(err instanceof Error ? err.message : String(err));
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
    toast.success('Credential deleted');
    deleteId.value = null;
  } catch (err) {
    toast.error(err instanceof Error ? err.message : String(err));
  }
}

// ── Public API (for the header "+" dropdown) ───────────────────────────────

const openImport = () => credentialFormDialogRef.value?.openAdd('key');
const openAddPassword = () => credentialFormDialogRef.value?.openAdd('password');

defineExpose({ openImport, openAddPassword });
</script>

<template>
  <div data-tauri-drag-region>
    <SidebarGroup class="pt-0 sticky top-0 z-10 bg-sidebar" data-tauri-drag-region>
      <SidebarGroupLabel class="px-4.5 text-sm text-sidebar-foreground" data-tauri-drag-region
        >Credentials</SidebarGroupLabel
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
      v-if="(query.trim() ? visibleGroups.length : groupedCredentials.length) === 0"
      class="flex flex-col justify-center gap-2 px-6 py-8"
      data-tauri-drag-region
    >
      <div class="space-y-1" data-tauri-drag-region>
        <p class="text-sm font-medium" data-tauri-drag-region>
          {{ query.trim() ? 'No credentials match' : 'No credentials yet' }}
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
        v-for="[groupName, groupCredentials] in visibleGroups"
        :key="groupName"
        data-tauri-drag-region
      >
        <SidebarGroupLabel class="px-4.5" data-tauri-drag-region>{{ groupName }}</SidebarGroupLabel>
        <SidebarGroupContent data-tauri-drag-region>
          <SidebarMenu class="gap-0.5" data-tauri-drag-region>
            <SidebarMenuItem
              v-for="credential in groupCredentials"
              :key="credential.id"
              class="mx-2 group/item"
              data-tauri-drag-region
            >
              <Tooltip>
                <TooltipTrigger as-child>
                  <SidebarMenuButton
                    size="lg"
                    class="hover:bg-accent dark:hover:bg-accent/50 transition-colors rounded-lg h-fit group/button px-2.5 py-[5.5px] items-start gap-1.5"
                    @click="credentialFormDialogRef?.openEdit(credential.id)"
                  >
                    <component
                      :is="credential.kind === 'key' ? KeyIcon : LockIcon"
                      class="size-3.5! shrink-0 text-muted-foreground group-hover/button:text-foreground transition-colors mt-0.5 -ml-px"
                    />
                    <div
                      class="flex flex-col gap-0.5 min-w-0 text-muted-foreground group-hover/button:text-foreground transition-colors"
                    >
                      <span class="truncate">{{ credential.name }}</span>
                      <span
                        class="truncate text-xs text-muted-foreground -mt-4 group-hover/button:mt-0 opacity-0 group-hover/button:opacity-100 transition-all duration-200 ease-in-out"
                      >
                        {{ credentialSubtitle(credential) }}
                      </span>
                    </div>
                  </SidebarMenuButton>
                </TooltipTrigger>
                <TooltipContent as="div" class="flex flex-col items-start gap-0.5">
                  <span class="text-xs">{{ credential.name }}</span>
                  <span class="text-xs opacity-50">{{ credentialSubtitle(credential) }}</span>
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
                    <DropdownMenuItem @click="openAssign(credential.id)">
                      <LinkIcon class="size-3.5" />
                      <span>Assign to host…</span>
                    </DropdownMenuItem>
                    <DropdownMenuItem @click="credentialFormDialogRef?.openEdit(credential.id)">
                      <EditIcon class="size-3.5" />
                      <span>Edit</span>
                    </DropdownMenuItem>
                    <DropdownMenuItem variant="destructive" @click="deleteId = credential.id">
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

    <CredentialFormDialog ref="credentialFormDialog" />

    <ResponsiveModal v-model:open="assignDialogOpen" v-slot="{ kind }">
      <ResponsiveModalContent :class="kind === 'dialog' ? 'max-w-md' : ''">
        <ResponsiveModalHeader>
          <ResponsiveModalTitle>Assign to host</ResponsiveModalTitle>
          <ResponsiveModalDescription>
            Assign
            <span class="text-foreground">{{ assigningCredential?.name }}</span> to a host.
          </ResponsiveModalDescription>
        </ResponsiveModalHeader>
        <div
          class="space-y-4 overflow-y-auto py-1"
          :class="kind === 'dialog' ? 'max-h-[60svh] -mx-6! px-6!' : '-mx-2! px-6!'"
        >
          <Field>
            <FieldLabel>Host</FieldLabel>
            <SelectHost v-model="assignHostId" placeholder="Select a host…" />
          </Field>
        </div>

        <ResponsiveModalFooter class="px-0" :class="kind === 'drawer' ? 'flex-col-reverse' : ''">
          <ResponsiveModalClose as-child>
            <Button variant="outline">Cancel</Button>
          </ResponsiveModalClose>
          <Button :disabled="!assignHostId" @click="handleAssign">Assign</Button>
        </ResponsiveModalFooter>
      </ResponsiveModalContent>
    </ResponsiveModal>

    <AlertDialog v-model:open="deleteConfirmDialogOpen">
      <AlertDialogContent>
        <AlertDialogHeader>
          <AlertDialogTitle>Delete credential?</AlertDialogTitle>
          <AlertDialogDescription>
            This credential will be permanently deleted from encrypted storage.
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
