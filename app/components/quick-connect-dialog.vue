<script setup lang="ts">
import { KeyIcon } from '@lucide/vue';

const open = defineModel<boolean>('open');

const { groupedHosts } = useHosts();
const { connect, connectDirect } = useSessions();

const passwordDialogOpen = ref(false);
const password = ref('');
const authMethod = ref<'password' | 'key'>('password');
const keyName = ref('');
const pendingDirectHost = ref<{ host: string; port: number; username: string } | null>(null);

const sshKeysManagerRef = useTemplateRef('sshKeysManager');
const commandInputRef = useTemplateRef('commandInput');

function parseInput(input: string): { host: string; port: number; username: string } | null {
  let remaining = input.trim();
  if (!remaining) return null;

  let username = 'root';
  let host: string;
  let port = 22;

  if (remaining.includes('@')) {
    const [u, rest] = remaining.split('@');
    username = u!;
    remaining = rest!;
  }

  if (remaining.includes(':')) {
    const [h, p] = remaining.split(':');
    host = h!;
    const parsedPort = Number.parseInt(p!, 10);
    if (!Number.isNaN(parsedPort) && parsedPort > 0 && parsedPort <= 65535) {
      port = parsedPort;
    }
  } else {
    host = remaining;
  }

  if (!host) return null;
  return { host, port, username };
}

const commandInputSearch = ref('');

const commandInputSearchInputEventListenerHandler = (event: InputEvent) => {
  commandInputSearch.value = (event.target as HTMLInputElement).value;
};

function commandInputSearchKeydownEventListenerHandler(event: KeyboardEvent) {
  if (event.key === 'Enter' && directMatch.value) {
    event.preventDefault();
    event.stopPropagation();
    handleDirectConnect();
  }
}

watch(
  commandInputRef,
  (value) => {
    const inputEl = value?.$el.querySelector('input') as HTMLInputElement | undefined;
    if (inputEl) {
      inputEl.addEventListener('input', commandInputSearchInputEventListenerHandler);
      inputEl.addEventListener('keydown', commandInputSearchKeydownEventListenerHandler);
    }
  },
  { immediate: true },
);

onBeforeUnmount(() => {
  const inputEl = commandInputRef.value?.$el.querySelector('input') as HTMLInputElement | undefined;
  if (inputEl) {
    inputEl.removeEventListener('input', commandInputSearchInputEventListenerHandler);
    inputEl.removeEventListener('keydown', commandInputSearchKeydownEventListenerHandler);
  }
});

const directMatch = computed(() => {
  return parseInput(commandInputSearch.value);
});

function handleDirectConnect() {
  const d = directMatch.value;
  if (!d) return;
  open.value = false;
  pendingDirectHost.value = d;
  password.value = '';
  authMethod.value = 'password';
  keyName.value = '';
  passwordDialogOpen.value = true;
}

function handleSavedHost(hostId: string) {
  open.value = false;
  connect(hostId).catch((e) => {
    toast.error(String(e));
  });
}

async function doConnect() {
  const host = pendingDirectHost.value;
  if (!host) return;
  passwordDialogOpen.value = false;
  const auth =
    authMethod.value === 'key'
      ? { type: 'key' as const, value: keyName.value }
      : { type: 'password' as const, value: password.value };
  connectDirect(host.host, host.port, host.username, auth).catch((error) => {
    toast.error(String(error));
  });
}
</script>

<template>
  <CommandDialog
    v-model:open="open"
    title="Quick Connect"
    description="Search saved hosts or type user@host to connect."
    class="**:data-[slot='command-input-wrapper']:pb-2"
  >
    <CommandInput ref="commandInput" placeholder="Search hosts or type user@host…" />
    <CommandList>
      <CommandEmpty>
        <template v-if="directMatch">
          Press Enter to connect to
          <span class="font-medium"
            >{{ directMatch.username }}@{{ directMatch.host }}:{{ directMatch.port }}</span
          >
        </template>
        <template v-else>No results.</template>
      </CommandEmpty>

      <CommandGroup
        v-for="[groupName, groupHosts] in groupedHosts"
        :heading="groupName"
        class="**:data-[slot='command-group-heading']:text-muted-foreground **:data-[slot='command-group-heading']:px-2 **:data-[slot='command-group-heading']:py-1.5 **:data-[slot='command-group-heading']:text-xs **:data-[slot='command-group-heading']:font-medium"
      >
        <CommandItem
          v-for="host in groupHosts"
          :key="host.id"
          :value="host.id"
          @select="handleSavedHost(host.id)"
        >
          <div class="flex flex-col gap-0.5 min-w-0">
            <span class="truncate text-sm">{{ host.name }}</span>
            <span class="truncate text-xs text-muted-foreground">
              {{ host.username }}@{{ host.host }}
            </span>
          </div>
        </CommandItem>
      </CommandGroup>
    </CommandList>
  </CommandDialog>

  <Dialog v-model:open="passwordDialogOpen">
    <DialogContent class="max-w-sm">
      <DialogHeader>
        <DialogTitle>Quick Connect</DialogTitle>
        <DialogDescription>
          {{ pendingDirectHost?.username }}@{{ pendingDirectHost?.host }}:{{
            pendingDirectHost?.port
          }}
        </DialogDescription>
      </DialogHeader>
      <div class="space-y-4 overflow-y-auto py-1">
        <Field>
          <FieldLabel for="direct-auth">Authentication Method</FieldLabel>
          <Select v-model="authMethod">
            <SelectTrigger id="direct-auth">
              <SelectValue />
            </SelectTrigger>
            <SelectContent>
              <SelectGroup>
                <SelectItem value="password">Password</SelectItem>
                <SelectItem value="key">SSH Key</SelectItem>
              </SelectGroup>
            </SelectContent>
          </Select>
        </Field>
        <Field v-if="authMethod === 'password'">
          <FieldLabel for="direct-password">Password</FieldLabel>
          <Input
            id="direct-password"
            :model-value="password || undefined"
            @update:model-value="password = String($event)"
            type="password"
            placeholder="••••••••"
            @keydown.enter="doConnect"
          />
        </Field>
        <Field v-if="authMethod === 'key'">
          <FieldLabel for="direct-key-name">SSH Key</FieldLabel>
          <Button
            id="direct-key-name"
            variant="outline"
            class="w-full justify-start font-normal"
            type="button"
            @click="sshKeysManagerRef?.open()"
          >
            <KeyIcon class="size-4" />
            {{ keyName || 'Select a key…' }}
          </Button>
        </Field>
      </div>
      <DialogFooter>
        <DialogClose as-child>
          <Button variant="ghost">Cancel</Button>
        </DialogClose>
        <Button :disabled="authMethod === 'key' && !keyName" @click="doConnect">Connect</Button>
      </DialogFooter>
    </DialogContent>
  </Dialog>

  <SshKeysManager ref="sshKeysManager" selectable @select="keyName = $event" />
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
