<script setup lang="ts">
const props = withDefaults(
  defineProps<{
    /** Default kind used when `openAdd()` is called without an argument. */
    kind?: 'key' | 'password';
  }>(),
  {},
);

const { credentials, create, update } = useCredentials();

type Credential = NonNullable<typeof credentials.value>[number];

const emit = defineEmits<{
  saved: [credential: Credential];
}>();

const modalOpen = ref(false);
const editingId = ref<string | null>(null);
const formKind = ref<'key' | 'password'>('password');

const formName = ref('');
const formValue = ref('');
const formPassphrase = ref('');
const formGroup = ref('');
const formTags = ref<string[]>([]);
const formError = ref('');
const saving = ref(false);

function resetForm() {
  formName.value = '';
  formValue.value = '';
  formPassphrase.value = '';
  formGroup.value = '';
  formTags.value = [];
  formError.value = '';
}

function openAdd(kind?: 'key' | 'password') {
  editingId.value = null;
  formKind.value = kind ?? props.kind ?? 'password';
  resetForm();
  modalOpen.value = true;
}

function openEdit(id: string) {
  const credential = (credentials.value ?? []).find((credential) => credential.id === id);
  if (!credential) return;
  editingId.value = credential.id;
  formKind.value = credential.kind;
  formName.value = credential.name;
  formValue.value = '';
  formPassphrase.value = '';
  formGroup.value = credential.group ?? '';
  formTags.value = [...credential.tags];
  formError.value = '';
  modalOpen.value = true;
}

function close() {
  modalOpen.value = false;
}

async function handleSubmit() {
  formError.value = '';
  const name = formName.value.trim();
  const group = formGroup.value.trim();

  if (!name) {
    formError.value = 'Name is required';
    return;
  }

  const isKey = formKind.value === 'key';
  if (!editingId.value && !formValue.value.trim()) {
    formError.value = isKey ? 'Private key content is required' : 'Password is required';
    return;
  }

  saving.value = true;
  try {
    let saved: Credential;
    if (editingId.value) {
      saved = await update(editingId.value, {
        name,
        value: formValue.value.trim() || undefined,
        keyPassphraseValue: isKey ? formPassphrase.value || undefined : undefined,
        group: group || null,
        tags: formTags.value,
      });
      toast.success('Credential updated');
    } else {
      saved = await create({
        name,
        kind: formKind.value,
        value: formValue.value.trim(),
        keyPassphraseValue: isKey ? formPassphrase.value || undefined : undefined,
        group: group || null,
        tags: formTags.value,
      });
      toast.success(isKey ? 'SSH key imported' : 'Password added');
    }
    modalOpen.value = false;
    emit('saved', saved);
  } catch (err) {
    formError.value = err instanceof Error ? err.message : String(err);
  } finally {
    saving.value = false;
  }
}

defineExpose({ openAdd, openEdit, close });
</script>

<template>
  <ResponsiveModal v-model:open="modalOpen" v-slot="{ kind }">
    <ResponsiveModalContent :class="kind === 'dialog' ? 'max-w-md' : ''">
      <ResponsiveModalHeader>
        <ResponsiveModalTitle>
          {{
            editingId ? 'Edit Credential' : formKind === 'key' ? 'Import SSH Key' : 'Add Password'
          }}
        </ResponsiveModalTitle>
        <ResponsiveModalDescription>
          {{
            formKind === 'key'
              ? 'Paste your private key content and give it a name.'
              : 'Create a reusable password credential.'
          }}
        </ResponsiveModalDescription>
      </ResponsiveModalHeader>

      <div
        class="space-y-4 overflow-y-auto py-1"
        :class="kind === 'dialog' ? 'max-h-[60svh] -mx-6! px-6!' : '-mx-2! px-6!'"
      >
        <Field>
          <FieldLabel for="credential-name">Name</FieldLabel>
          <Input
            id="credential-name"
            v-model="formName"
            :placeholder="formKind === 'key' ? 'laptop-ed25519' : 'Production DB'"
            :disabled="saving"
          />
        </Field>
        <Field v-if="formKind === 'key'">
          <FieldLabel for="credential-value">Private Key</FieldLabel>
          <Textarea
            id="credential-value"
            v-model="formValue"
            :disabled="saving"
            :placeholder="
              editingId
                ? 'Leave blank to keep the existing key'
                : 'Paste your OpenSSH private key here…'
            "
            rows="6"
            class="max-h-40"
            :class="!!formValue.trim() ? 'font-mono' : ''"
          />
          <FieldDescription>Ed25519 and RSA keys are supported.</FieldDescription>
        </Field>
        <Field v-else>
          <FieldLabel for="credential-value">Password</FieldLabel>
          <Input
            id="credential-value"
            v-model="formValue"
            type="password"
            :disabled="saving"
            :placeholder="editingId ? 'Leave blank to keep the existing password' : '••••••••'"
          />
        </Field>
        <Field v-if="formKind === 'key' && (!editingId || formValue.trim())">
          <FieldLabel for="credential-passphrase">Passphrase</FieldLabel>
          <Input
            id="credential-passphrase"
            v-model="formPassphrase"
            type="password"
            :disabled="saving"
            placeholder="Optional"
          />
        </Field>
        <Field>
          <FieldLabel for="credential-group">Group</FieldLabel>
          <Input
            id="credential-group"
            v-model="formGroup"
            placeholder="Optional"
            :disabled="saving"
          />
        </Field>
        <Field>
          <FieldLabel>Tags</FieldLabel>
          <TagsInput v-model="formTags" class="py-1.75" data-slot="tags-input-wrapper">
            <TagsInputItem v-for="tag in formTags" :key="tag" :value="tag">
              <TagsInputItemText>{{ tag }}</TagsInputItemText>
              <TagsInputItemDelete />
            </TagsInputItem>
            <TagsInputInput placeholder="web, db…" />
          </TagsInput>
        </Field>

        <p v-if="formError" class="text-destructive text-sm">
          {{ formError }}
        </p>
      </div>

      <ResponsiveModalFooter :class="kind === 'drawer' ? 'flex-col-reverse' : ''">
        <ResponsiveModalClose as-child>
          <Button variant="outline" :disabled="saving">Cancel</Button>
        </ResponsiveModalClose>
        <Button :disabled="saving" @click="handleSubmit">
          {{
            saving
              ? 'Saving…'
              : editingId
                ? 'Save'
                : formKind === 'key'
                  ? 'Import Key'
                  : 'Add Password'
          }}
        </Button>
      </ResponsiveModalFooter>
    </ResponsiveModalContent>
  </ResponsiveModal>
</template>
