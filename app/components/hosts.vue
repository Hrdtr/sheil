<script setup lang="ts">
const {
  hosts,
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
  keyId: null,
  passwordId: null,
  tags: [],
});

watch(
  () => formState.value.authMethod,
  (authMethod) => {
    if (authMethod === 'none') {
      formState.value.keyId = null;
      formState.value.passwordId = null;
    }
  },
);

const createHost = async () => {
  const { name, host, username, authMethod, keyId } = formState.value;
  if (!name?.trim()) return;
  if (!host?.trim()) return;
  if (!username?.trim()) return;
  if (authMethod === 'key' && !keyId) return;

  try {
    await _createHost({
      name: name?.trim(),
      host: host?.trim(),
      port: formState.value.port,
      username: username?.trim(),
      protocol: 'ssh',
      group: formState.value.group?.trim() || undefined,
      authMethod: formState.value.authMethod,
      keyId: authMethod === 'key' ? (keyId ?? null) : null,
      passwordId: authMethod === 'password' ? (formState.value.passwordId ?? null) : null,
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
  const { name, host, username, authMethod, keyId } = formState.value;
  if (!updateHostId.value) return;
  if (!name?.trim()) return;
  if (!host?.trim()) return;
  if (!username?.trim()) return;
  if (authMethod === 'key' && !keyId) return;

  try {
    await _updateHost(updateHostId.value, {
      name: name.trim() || undefined,
      host: host.trim() || undefined,
      port: formState.value.port,
      username: username.trim() || undefined,
      protocol: 'ssh',
      group: formState.value.group?.trim() || undefined,
      authMethod: formState.value.authMethod,
      keyId: authMethod === 'key' ? (keyId ?? null) : null,
      passwordId: authMethod === 'password' ? (formState.value.passwordId ?? null) : null,
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
          keyId: null,
          passwordId: null,
          tags: [],
        };
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
          keyId: host.keyId ?? null,
          passwordId: host.passwordId ?? null,
          tags: host.tags ?? [],
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
        <Field>
          <FieldLabel for="host-username">Username</FieldLabel>
          <Input id="host-username" v-model="formState.username" placeholder="root" required />
        </Field>
        <Field>
          <FieldLabel for="host-auth">Authentication Method</FieldLabel>
          <Select v-model="formState.authMethod">
            <SelectTrigger id="host-auth">
              <SelectValue />
            </SelectTrigger>
            <SelectContent>
              <SelectGroup>
                <SelectItem value="none">None</SelectItem>
                <SelectItem value="password">Password</SelectItem>
                <SelectItem value="key">SSH Key</SelectItem>
              </SelectGroup>
            </SelectContent>
          </Select>
        </Field>
        <Field v-if="formState.authMethod === 'password'">
          <FieldLabel>Password</FieldLabel>
          <SelectCredential
            v-model="formState.passwordId"
            kind="password"
            placeholder="Select a password…"
          />
        </Field>
        <Field v-if="formState.authMethod === 'key'">
          <FieldLabel>SSH Key</FieldLabel>
          <SelectCredential v-model="formState.keyId" kind="key" placeholder="Select a key…" />
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
        <Field>
          <FieldLabel>Tags</FieldLabel>
          <TagsInput v-model="formState.tags" class="py-1.75" data-slot="tags-input-wrapper">
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
          <Button variant="outline">Cancel</Button>
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
          The host configuration will be permanently deleted.
        </AlertDialogDescription>
      </AlertDialogHeader>
      <AlertDialogFooter>
        <AlertDialogCancel variant="outline">Cancel</AlertDialogCancel>
        <AlertDialogAction @click="deleteHost" variant="destructive">Delete</AlertDialogAction>
      </AlertDialogFooter>
    </AlertDialogContent>
  </AlertDialog>
</template>
