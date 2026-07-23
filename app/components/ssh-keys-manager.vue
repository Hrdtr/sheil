<script setup lang="ts">
import { KeyIcon, PlusIcon, TrashIcon } from '@lucide/vue';

const props = defineProps<{
  /** When true, clicking a key row emits "select" and closes the modal. */
  selectable?: boolean;
}>();

const emit = defineEmits<{
  imported: [];
  deleted: [keyName: string];
  select: [keyName: string];
}>();

const { hosts } = useHosts();
const { keys, importKey, deleteKey } = useSshKeys();

const modalOpen = ref(false);

// Derive key usage from hosts so we can warn when a key is still in use.
const keyUsage = computed(() => {
  const map = new Map<string, { hosts: Array<{ id: string; name: string }> }>();
  for (const host of hosts.value ?? []) {
    if (!host.keyName) continue;
    const entry = map.get(host.keyName) ?? { hosts: [] };
    entry.hosts.push({ id: host.id, name: host.name });
    map.set(host.keyName, entry);
  }
  return map;
});

// ── Import form (collapsible) ──────────────────────────────────────────────

const showImportForm = ref(false);
const importName = ref('');
const importKeyData = ref('');
const importPassphrase = ref('');
const importError = ref('');
const importing = ref(false);

const handleImport = async () => {
  importError.value = '';
  const name = importName.value.trim();
  const data = importKeyData.value.trim();

  if (!name) {
    importError.value = 'Key name is required';
    return;
  }
  if (!data) {
    importError.value = 'Private key content is required';
    return;
  }

  importing.value = true;
  try {
    const info = await importKey(name, data, importPassphrase.value || undefined);
    toast.success(`Key "${info.name}" imported (${info.keyType}, ${info.fingerprint})`);
    importName.value = '';
    importKeyData.value = '';
    importPassphrase.value = '';
    showImportForm.value = false;
    emit('imported');
  } catch (err) {
    importError.value = err instanceof Error ? err.message : String(err);
  } finally {
    importing.value = false;
  }
};

const onSelect = (keyName: string) => {
  if (!props.selectable) return;
  emit('select', keyName);
  modalOpen.value = false;
};

// ── Delete ─────────────────────────────────────────────────────────────────

const deleteSshKeyName = ref<string | null>(null);
const deleteSshKeyConfirmDialogOpen = ref(false);

watch(deleteSshKeyName, (value) => {
  deleteSshKeyConfirmDialogOpen.value = !!value;
});
watch(deleteSshKeyConfirmDialogOpen, (value) => {
  if (!value) {
    setTimeout(() => {
      deleteSshKeyName.value = null;
    }, 300);
  }
});

const handleDelete = async () => {
  const keyName = deleteSshKeyName.value;
  if (!keyName) return;

  try {
    await deleteKey(keyName);
    toast.success(`Key "${keyName}" deleted`);
    emit('deleted', keyName);
    deleteSshKeyName.value = null;
  } catch (err) {
    toast.error(err instanceof Error ? err.message : String(err));
  }
};

// ── Public API ─────────────────────────────────────────────────────────────

const open = () => {
  showImportForm.value = false;
  modalOpen.value = true;
};
const close = () => {
  modalOpen.value = false;
};

defineExpose({ open, close });
</script>

<template>
  <ResponsiveModal v-model:open="modalOpen" v-slot="{ kind }">
    <ResponsiveModalContent :class="kind === 'dialog' ? 'max-w-md' : ''">
      <ResponsiveModalHeader>
        <template v-if="showImportForm">
          <ResponsiveModalTitle>Import SSH Key</ResponsiveModalTitle>
          <ResponsiveModalDescription>
            Paste your private key content and give it a name.
          </ResponsiveModalDescription>
        </template>
        <template v-else>
          <ResponsiveModalTitle>SSH Keys</ResponsiveModalTitle>
          <ResponsiveModalDescription> Manage your SSH keys. </ResponsiveModalDescription>
        </template>
      </ResponsiveModalHeader>

      <div
        class="space-y-4 overflow-y-auto py-1"
        :class="kind === 'dialog' ? 'max-h-[60svh] -mx-6! px-6!' : '-mx-2! px-6!'"
      >
        <!-- Import form (replaces list when visible) -->
        <template v-if="showImportForm">
          <Field>
            <FieldLabel for="import-key-name">Key Name</FieldLabel>
            <Input
              id="import-key-name"
              v-model="importName"
              placeholder="laptop-ed25519"
              :disabled="importing"
            />
          </Field>
          <Field>
            <FieldLabel for="import-key-data">Private Key</FieldLabel>
            <Textarea
              id="import-key-data"
              v-model="importKeyData"
              :disabled="importing"
              placeholder="Paste your OpenSSH private key here…"
              rows="6"
              class="font-mono max-h-40"
            />
            <FieldDescription> Ed25519 and RSA keys are supported. </FieldDescription>
          </Field>
          <Field>
            <FieldLabel for="import-key-passphrase">Passphrase</FieldLabel>
            <Input
              id="import-key-passphrase"
              v-model="importPassphrase"
              type="password"
              :disabled="importing"
            />
            <FieldDescription> Leave blank if the key has no passphrase </FieldDescription>
          </Field>

          <p v-if="importError" class="text-destructive text-sm">
            {{ importError }}
          </p>
        </template>

        <!-- Key list (hidden when import form is open) -->
        <template v-else>
          <Button type="button" variant="outline" class="w-full" @click="showImportForm = true">
            <PlusIcon class="size-4" />
            Import Key
          </Button>

          <Separator />

          <div
            v-if="(keys?.length ?? 0) === 0"
            class="flex flex-col items-center gap-2 py-6 text-muted-foreground"
          >
            <KeyIcon class="size-8 opacity-40" />
            <p class="text-sm">No SSH keys imported yet.</p>
            <p class="text-xs">Import a key above to use it for key-based authentication.</p>
          </div>

          <FieldSet v-else>
            <FieldLegend variant="label">Imported Keys</FieldLegend>
            <FieldGroup class="gap-3">
              <div
                v-for="key in keys"
                :key="key.name"
                class="flex items-start justify-between rounded-md bg-accent/50 transition-colors px-3 py-2.5"
                :class="selectable ? 'cursor-pointer hover:bg-accent' : ''"
                @click="onSelect(key.name)"
              >
                <div class="min-w-0 flex-1">
                  <p class="truncate text-sm mb-1">{{ key.name }}</p>
                  <p class="font-mono text-muted-foreground text-xs">
                    {{ key.keyType }}
                    <template v-if="keyUsage.get(key.name)?.hosts.length">
                      · {{ keyUsage.get(key.name)?.hosts.length }}
                      {{ keyUsage.get(key.name)?.hosts.length === 1 ? 'host' : 'hosts' }}
                    </template>
                  </p>
                  <p class="truncate font-mono text-muted-foreground text-xs">
                    {{ key.fingerprint }}
                  </p>
                </div>
                <Button
                  variant="outline"
                  size="icon-sm"
                  class="shrink-0 text-muted-foreground hover:text-destructive"
                  @click.stop="deleteSshKeyName = key.name"
                >
                  <TrashIcon class="size-4" />
                </Button>

                <!-- Delete confirmation -->
                <AlertDialog v-model:open="deleteSshKeyConfirmDialogOpen">
                  <AlertDialogContent>
                    <AlertDialogHeader>
                      <AlertDialogTitle>Delete SSH key?</AlertDialogTitle>
                      <AlertDialogDescription>
                        This will delete
                        <strong>{{ deleteSshKeyName }}</strong> from encrypted storage.
                        <template v-if="deleteSshKeyName && keyUsage.get(deleteSshKeyName)">
                          <br /><br />
                          <span class="font-medium text-destructive">Warning:</span>
                          This key is used by
                          {{
                            (keyUsage.get(deleteSshKeyName)?.hosts.length ?? 0) === 1
                              ? '1 host'
                              : `${keyUsage.get(deleteSshKeyName)?.hosts.length} hosts`
                          }}
                          hosts. Hosts using this key will fall back to password authentication.
                        </template>
                      </AlertDialogDescription>
                    </AlertDialogHeader>
                    <AlertDialogFooter>
                      <AlertDialogCancel variant="ghost">Cancel</AlertDialogCancel>
                      <AlertDialogAction @click="handleDelete" variant="destructive"
                        >Delete</AlertDialogAction
                      >
                    </AlertDialogFooter>
                  </AlertDialogContent>
                </AlertDialog>
              </div>
            </FieldGroup>
          </FieldSet>
        </template>
      </div>

      <ResponsiveModalFooter :class="kind === 'drawer' ? 'flex-col-reverse' : ''">
        <template v-if="showImportForm">
          <Button variant="outline" :disabled="importing" @click="showImportForm = false">
            Cancel
          </Button>
          <Button :disabled="importing" @click="handleImport">
            {{ importing ? 'Importing…' : 'Import Key' }}
          </Button>
        </template>
        <template v-else>
          <ResponsiveModalClose as-child>
            <Button variant="outline"> Close </Button>
          </ResponsiveModalClose>
        </template>
      </ResponsiveModalFooter>
    </ResponsiveModalContent>
  </ResponsiveModal>
</template>
