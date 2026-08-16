<script setup lang="ts">
import { CheckIcon, ChevronsUpDownIcon, KeyIcon, LockIcon, PlusIcon } from '@lucide/vue';

const props = withDefaults(
  defineProps<{
    /** Filter the list to a single credential kind. */
    kind?: 'key' | 'password';
    placeholder?: string;
  }>(),
  { placeholder: 'Select a credential…' },
);

const model = defineModel<string | null>({ default: null });

const { credentials, groupedCredentials } = useCredentials();

type Credential = NonNullable<typeof credentials.value>[number];

const filteredCredentials = computed(() => {
  if (!props.kind) return credentials.value ?? [];
  return (credentials.value ?? []).filter((credential) => credential.kind === props.kind);
});

const selected = computed(
  () => filteredCredentials.value.find((credential) => credential.id === model.value) ?? null,
);

function getDisplayValue(id: string | null): string {
  if (!id) return '';
  return (credentials.value ?? []).find((credential) => credential.id === id)?.name ?? '';
}

const formDialogRef = useTemplateRef('formDialog');

function openCreate(kind: 'key' | 'password') {
  formDialogRef.value?.openAdd(kind);
}

function onSaved(credential: Credential) {
  model.value = credential.id;
}
</script>

<template>
  <Combobox v-model="model" class="w-full">
    <ComboboxAnchor as-child>
      <ComboboxTrigger as-child>
        <Button variant="outline" class="w-full justify-between font-normal">
          <span class="flex min-w-0 items-center gap-2">
            <component
              v-if="selected"
              :is="selected.kind === 'key' ? KeyIcon : LockIcon"
              class="size-4 shrink-0"
            />
            <span class="truncate" :class="!selected ? 'text-muted-foreground' : ''">{{
              selected?.name ?? placeholder
            }}</span>
          </span>
          <ChevronsUpDownIcon class="size-4 shrink-0 opacity-50" />
        </Button>
      </ComboboxTrigger>
    </ComboboxAnchor>
    <ComboboxList>
      <ComboboxInput
        :display-value="getDisplayValue"
        placeholder="Search credentials…"
        class="text-sm"
      />
      <ComboboxViewport class="max-h-64">
        <ComboboxEmpty>No credentials found.</ComboboxEmpty>
        <ComboboxGroup>
          <ComboboxItem
            v-if="!kind || kind === 'key'"
            value="create-key"
            @select.prevent="openCreate('key')"
          >
            <PlusIcon class="size-3.5 shrink-0" />
            <span>Create new key credential</span>
          </ComboboxItem>
          <ComboboxItem
            v-if="!kind || kind === 'password'"
            value="create-password"
            @select.prevent="openCreate('password')"
          >
            <PlusIcon class="size-3.5 shrink-0" />
            <span>Create new password credential</span>
          </ComboboxItem>
        </ComboboxGroup>
        <ComboboxSeparator />
        <ComboboxGroup
          v-for="[groupName, groupCredentials] in groupedCredentials"
          :key="groupName"
          :heading="groupName"
        >
          <ComboboxItem
            v-for="credential in groupCredentials"
            :key="credential.id"
            :value="credential.id"
          >
            <component
              :is="credential.kind === 'key' ? KeyIcon : LockIcon"
              class="size-3.5 shrink-0"
            />
            <span class="truncate">{{ credential.name }}</span>
            <ComboboxItemIndicator>
              <CheckIcon class="size-4" />
            </ComboboxItemIndicator>
          </ComboboxItem>
        </ComboboxGroup>
      </ComboboxViewport>

      <CredentialFormDialog ref="formDialog" :kind="kind" @saved="onSaved" />
    </ComboboxList>
  </Combobox>
</template>
