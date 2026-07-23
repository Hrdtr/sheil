<script setup lang="ts">
import { KeyIcon } from '@lucide/vue';

const {
  hosts,
  refreshHosts,
  createHost: _createHost,
  updateHost: _updateHost,
  deleteHost: _deleteHost,
} = useHosts();

type HostCreateInput = Parameters<typeof _createHost>[0];
type HostUpdateInput = Parameters<typeof _updateHost>[1];

const formModalOpen = ref(false);
const formModalSubmitButton = useTemplateRef('formModalSubmitButtonRef');
const formState = ref<HostCreateInput | HostUpdateInput>({
  name: '',
  host: '',
  port: 22,
  username: '',
  protocol: 'ssh',
  group: '',
  authMethod: 'password',
  keyName: '',
  password: '',
  tags: [],
});
const clearPassword = ref(false);
const storedHost = ref<{
  authMethod: string;
  keyName: string | null;
  hasPassword: boolean;
} | null>(null);

const sshKeysManagerRef = useTemplateRef('sshKeysManager');

const createHost = async () => {
  const { name, host, username, authMethod, keyName } = formState.value;
  if (!name?.trim()) return;
  if (!host?.trim()) return;
  if (!username?.trim()) return;
  if (authMethod === 'key' && !keyName?.trim()) return;

  try {
    await _createHost({
      name: name?.trim(),
      host: host?.trim(),
      port: formState.value.port,
      username: username?.trim(),
      protocol: 'ssh',
      group: formState.value.group?.trim() || undefined,
      authMethod: formState.value.authMethod,
      keyName: authMethod === 'key' ? keyName?.trim() : undefined,
      password: authMethod === 'password' ? formState.value.password || undefined : undefined,
      tags: formState.value.tags,
    } as HostCreateInput);
    toast.success('Host created');
    formModalOpen.value = false;
  } catch (err) {
    toast.error(err instanceof Error ? err.message : String(err));
  }
};

const updateHostId = ref<string | null>(null);
const updateHost = async () => {
  const { name, host, username, authMethod, keyName } = formState.value;
  if (!updateHostId.value) return;
  if (!name?.trim()) return;
  if (!host?.trim()) return;
  if (!username?.trim()) return;
  if (authMethod === 'key' && !keyName?.trim()) return;

  try {
    let passwordPayload: string | undefined;
    if (authMethod === 'password') {
      if (clearPassword.value) {
        passwordPayload = '';
      } else if (formState.value.password) {
        passwordPayload = formState.value.password;
      }
    }

    await _updateHost(updateHostId.value, {
      name: name.trim() || undefined,
      host: host.trim() || undefined,
      port: formState.value.port,
      username: username.trim() || undefined,
      protocol: 'ssh',
      group: formState.value.group?.trim() || undefined,
      authMethod: formState.value.authMethod,
      keyName: authMethod === 'key' ? keyName?.trim() : undefined,
      password: passwordPayload,
      tags: formState.value.tags,
    } as HostUpdateInput);
    toast.success('Host updated');
    formModalOpen.value = false;
  } catch (err) {
    toast.error(err instanceof Error ? err.message : String(err));
  }
};

const deleteHostId = ref<string | null>(null);
const deleteHostConfirmDialogOpen = ref(false);

watch(deleteHostId, (value) => {
  deleteHostConfirmDialogOpen.value = !!value;
});
watch(deleteHostConfirmDialogOpen, (value) => {
  if (!value) {
    setTimeout(() => {
      deleteHostId.value = null;
    }, 300);
  }
});
const deleteHost = async () => {
  if (!deleteHostId.value) return;

  try {
    await _deleteHost(deleteHostId.value);
    toast.success('Host deleted');
    deleteHostId.value = null;
  } catch (err) {
    toast.error(err instanceof Error ? err.message : String(err));
  }
};
</script>

<template>
  <slot
    v-bind="{
      hosts,
      createHost: () => {
        updateHostId = null;
        formState = {
          name: '',
          host: '',
          port: 22,
          username: '',
          protocol: 'ssh',
          group: '',
          authMethod: 'password',
          keyName: '',
          password: '',
          tags: [],
        };
        clearPassword = false;
        storedHost = null;
        formModalOpen = true;
      },
      updateHost: (host: NonNullable<typeof hosts>[number]) => {
        updateHostId = host.id;
        formState = {
          name: host.name,
          host: host.host,
          port: host.port,
          username: host.username,
          protocol: host.protocol,
          group: host.group ?? '',
          authMethod: host.authMethod,
          keyName: host.keyName ?? '',
          password: '',
          tags: host.tags ?? [],
        };
        clearPassword = false;
        storedHost = {
          authMethod: host.authMethod,
          keyName: host.keyName ?? null,
          hasPassword: host.hasPassword,
        };
        formModalOpen = true;
      },
      deleteHost: (hostId: string) => {
        deleteHostId = hostId;
      },
    }"
  />

  <ResponsiveModal v-model:open="formModalOpen" v-slot="{ kind }">
    <ResponsiveModalContent :class="kind === 'dialog' ? 'max-w-md' : ''">
      <ResponsiveModalHeader>
        <ResponsiveModalTitle>{{ updateHostId ? 'Update Host' : 'Add Host' }}</ResponsiveModalTitle>
        <ResponsiveModalDescription>
          {{
            updateHostId
              ? 'Edit the host connection details.'
              : 'Fill in the details for the new SSH host.'
          }}
        </ResponsiveModalDescription>
      </ResponsiveModalHeader>

      <form
        class="space-y-4 overflow-y-auto py-1"
        :class="kind === 'dialog' ? 'max-h-[60svh] -mx-6! px-6!' : '-mx-2! px-6!'"
        @submit.prevent="
          () => {
            if (updateHostId) updateHost();
            else createHost();
          }
        "
      >
        <Field>
          <FieldLabel for="host-name">Name</FieldLabel>
          <Input id="host-name" v-model="formState.name" placeholder="Production server" required />
        </Field>
        <div class="flex gap-3">
          <Field class="flex-1">
            <FieldLabel for="host-address">Host</FieldLabel>
            <Input id="host-address" v-model="formState.host" placeholder="192.168.1.10" required />
          </Field>
          <Field class="w-24">
            <FieldLabel for="host-port">Port</FieldLabel>
            <Input id="host-port" v-model.number="formState.port" type="number" />
          </Field>
        </div>
        <div class="grid grid-cols-2 gap-3">
          <Field>
            <FieldLabel for="host-username">Username</FieldLabel>
            <Input id="host-username" v-model="formState.username" placeholder="root" required />
          </Field>
          <Field>
            <FieldLabel for="host-group">Group</FieldLabel>
            <Input
              id="host-group"
              :model-value="formState.group || undefined"
              @update:model-value="formState.group = String($event)"
              placeholder="Optional"
            />
          </Field>
        </div>
        <Field>
          <FieldLabel for="host-auth">Authentication Method</FieldLabel>
          <Select v-model="formState.authMethod">
            <SelectTrigger id="host-auth">
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
        <Field v-if="formState.authMethod === 'password'">
          <FieldLabel for="host-password">
            Password
            <span v-if="storedHost" class="text-muted-foreground ml-1"
              >({{ storedHost.hasPassword ? 'saved — leave blank to keep' : 'none saved' }})</span
            >
          </FieldLabel>
          <Input
            id="host-password"
            :model-value="formState.password || undefined"
            @update:model-value="formState.password = String($event)"
            type="password"
            placeholder="••••••••"
            :disabled="clearPassword"
          />
          <div v-if="storedHost?.hasPassword" class="flex items-center gap-2">
            <Checkbox id="clear-password" v-model="clearPassword" />
            <Label for="clear-password" class="text-sm text-muted-foreground cursor-pointer">
              Remove stored password
            </Label>
          </div>
        </Field>
        <Field v-if="formState.authMethod === 'key'">
          <FieldLabel for="host-key-name">SSH Key</FieldLabel>
          <Button
            id="host-key-name"
            variant="outline"
            class="w-full justify-start font-normal"
            type="button"
            data-slot="select-trigger"
            @click="sshKeysManagerRef?.open()"
          >
            <KeyIcon class="size-4" />
            {{ formState.keyName || 'Select a key…' }}
          </Button>
        </Field>
        <Field>
          <FieldLabel>Tags</FieldLabel>
          <TagsInput v-model="formState.tags" class="py-2" data-slot="tags-input-wrapper">
            <TagsInputItem v-for="tag in formState.tags" :key="tag" :value="tag">
              <TagsInputItemText>{{ tag }}</TagsInputItemText>
              <TagsInputItemDelete />
            </TagsInputItem>
            <TagsInputInput placeholder="web, nginx…" />
          </TagsInput>
        </Field>

        <button ref="formModalSubmitButtonRef" type="submit" class="hidden" />
      </form>

      <ResponsiveModalFooter :class="kind === 'drawer' ? 'flex-col-reverse' : ''">
        <ResponsiveModalClose as-child>
          <Button variant="ghost">Cancel</Button>
        </ResponsiveModalClose>
        <Button @click="() => formModalSubmitButton?.click()">{{
          updateHostId ? 'Update' : 'Create'
        }}</Button>
      </ResponsiveModalFooter>
    </ResponsiveModalContent>
  </ResponsiveModal>

  <AlertDialog v-model:open="deleteHostConfirmDialogOpen">
    <AlertDialogContent>
      <AlertDialogHeader>
        <AlertDialogTitle>Delete host?</AlertDialogTitle>
        <AlertDialogDescription>
          The host configuration and its stored password will be permanently deleted.
        </AlertDialogDescription>
      </AlertDialogHeader>
      <AlertDialogFooter>
        <AlertDialogCancel variant="ghost">Cancel</AlertDialogCancel>
        <AlertDialogAction @click="deleteHost" variant="destructive">Delete</AlertDialogAction>
      </AlertDialogFooter>
    </AlertDialogContent>
  </AlertDialog>

  <SshKeysManager
    ref="sshKeysManager"
    selectable
    @select="formState.keyName = $event"
    @deleted="
      () => {
        formState.authMethod = 'password';
        formState.password = '';
        refreshHosts();
      }
    "
  />
</template>
