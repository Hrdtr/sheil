<script setup lang="ts">
import { SparklesIcon } from '@lucide/vue';

type Snippet = NonNullable<ReturnType<typeof useSnippets>['snippets']['value']>[number];
type SnippetPrefill = Partial<Pick<Snippet, 'name' | 'command' | 'description' | 'group' | 'tags'>>;

const { snippets, create, update } = useSnippets();
const { groupedHosts } = useHosts();
const { enabled: aiEnabled } = useAiSettings();
const { generateCommand } = useAiEngine();

const emit = defineEmits<{
  saved: [snippet: Snippet];
}>();

const modalOpen = ref(false);
const editingId = ref<string | null>(null);

const formName = ref('');
const formCommand = ref('');
const formDescription = ref('');
const formGroup = ref('');
const formTags = ref<string[]>([]);
const formScope = ref<'global' | 'host' | 'hostGroup'>('global');
const formHostId = ref<string | null>(null);
const formHostGroup = ref('');
const formError = ref('');
const saving = ref(false);

const aiPrompt = ref('');
const aiGenerating = ref(false);

const hostGroups = computed(() => groupedHosts.value.map(([groupName]) => groupName));

const modalDescription =
  'Save a reusable command. Use {{variable}} placeholders to be prompted or auto-filled at run time.';
const builtinVariablesHelp =
  'Built-in variables: {{host}}, {{hostname}}, {{username}}, {{port}}. Any other variable prompts for input before running.';

function resetForm() {
  formName.value = '';
  formCommand.value = '';
  formDescription.value = '';
  formGroup.value = '';
  formTags.value = [];
  formScope.value = 'global';
  formHostId.value = null;
  formHostGroup.value = '';
  formError.value = '';
  aiPrompt.value = '';
}

function openAdd(prefill?: SnippetPrefill) {
  editingId.value = null;
  resetForm();
  if (prefill) {
    formName.value = prefill.name ?? '';
    formCommand.value = prefill.command ?? '';
    formDescription.value = prefill.description ?? '';
    formGroup.value = prefill.group ?? '';
    formTags.value = [...(prefill.tags ?? [])];
  }
  modalOpen.value = true;
}

function openEdit(id: string) {
  const snippet = (snippets.value ?? []).find((s) => s.id === id);
  if (!snippet) return;
  editingId.value = snippet.id;
  formName.value = snippet.name;
  formCommand.value = snippet.command;
  formDescription.value = snippet.description ?? '';
  formGroup.value = snippet.group ?? '';
  formTags.value = [...snippet.tags];
  if (snippet.hostId) {
    formScope.value = 'host';
    formHostId.value = snippet.hostId;
    formHostGroup.value = '';
  } else if (snippet.hostGroup) {
    formScope.value = 'hostGroup';
    formHostGroup.value = snippet.hostGroup;
    formHostId.value = null;
  } else {
    formScope.value = 'global';
    formHostId.value = null;
    formHostGroup.value = '';
  }
  formError.value = '';
  aiPrompt.value = '';
  modalOpen.value = true;
}

function close() {
  modalOpen.value = false;
}

function aiErrorMessage(err: unknown): string {
  const message = err instanceof Error ? err.message : String(err);
  if (/model file not found|no model loaded/i.test(message)) {
    return 'AI model not downloaded. Download it in Settings → AI.';
  }
  return message;
}

async function generateWithAi() {
  const prompt = aiPrompt.value.trim();
  if (!prompt || aiGenerating.value) return;
  aiGenerating.value = true;
  formError.value = '';
  try {
    const result = await generateCommand(prompt);
    formCommand.value = result.trim();
    if (!formName.value.trim()) {
      formName.value = prompt;
    }
  } catch (err) {
    formError.value = aiErrorMessage(err);
    toast.error(formError.value);
  } finally {
    aiGenerating.value = false;
  }
}

async function handleSubmit() {
  formError.value = '';
  const name = formName.value.trim();
  const command = formCommand.value.trim();

  if (!name) {
    formError.value = 'Name is required';
    return;
  }
  if (!command) {
    formError.value = 'Command is required';
    return;
  }
  if (formScope.value === 'host' && !formHostId.value) {
    formError.value = 'Select a host or switch the scope';
    return;
  }
  if (formScope.value === 'hostGroup' && !formHostGroup.value.trim()) {
    formError.value = 'Select a host group or switch the scope';
    return;
  }

  const payload = {
    name,
    command,
    description: formDescription.value.trim() || null,
    group: formGroup.value.trim() || null,
    tags: formTags.value,
    hostId: formScope.value === 'host' ? formHostId.value : null,
    hostGroup: formScope.value === 'hostGroup' ? formHostGroup.value.trim() : null,
  };

  saving.value = true;
  try {
    let saved: Snippet;
    if (editingId.value) {
      saved = await update(editingId.value, payload);
      toast.success('Snippet updated');
    } else {
      saved = await create(payload);
      toast.success('Snippet created');
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
        <ResponsiveModalTitle>{{
          editingId ? 'Edit Snippet' : 'Add Snippet'
        }}</ResponsiveModalTitle>
        <ResponsiveModalDescription>{{ modalDescription }}</ResponsiveModalDescription>
      </ResponsiveModalHeader>

      <div
        class="space-y-4 overflow-y-auto py-1"
        :class="kind === 'dialog' ? 'max-h-[60svh] -mx-6! px-6!' : '-mx-2! px-6!'"
      >
        <Field>
          <FieldLabel for="snippet-name">Name</FieldLabel>
          <Input id="snippet-name" v-model="formName" placeholder="Disk usage" :disabled="saving" />
        </Field>

        <Field>
          <FieldLabel for="snippet-command">Command</FieldLabel>
          <Textarea
            id="snippet-command"
            v-model="formCommand"
            placeholder="df -h"
            rows="3"
            class="font-mono"
            :disabled="saving"
          />
          <FieldDescription>{{ builtinVariablesHelp }}</FieldDescription>
        </Field>

        <Field v-if="aiEnabled">
          <FieldLabel for="snippet-ai-prompt">Generate with AI</FieldLabel>
          <div class="flex gap-2">
            <Input
              id="snippet-ai-prompt"
              v-model="aiPrompt"
              placeholder="e.g. find files modified in the last 24h"
              :disabled="saving || aiGenerating"
              @keydown.enter.prevent="generateWithAi"
            />
            <Button
              variant="outline"
              class="shrink-0"
              :disabled="saving || aiGenerating || !aiPrompt.trim()"
              @click="generateWithAi"
            >
              <SparklesIcon class="size-3.5" />
              {{ aiGenerating ? 'Generating…' : 'Generate' }}
            </Button>
          </div>
        </Field>

        <Field>
          <FieldLabel for="snippet-description">Description</FieldLabel>
          <Input
            id="snippet-description"
            v-model="formDescription"
            placeholder="Optional"
            :disabled="saving"
          />
        </Field>

        <Field>
          <FieldLabel for="snippet-group">Group</FieldLabel>
          <Input id="snippet-group" v-model="formGroup" placeholder="Optional" :disabled="saving" />
        </Field>

        <Field>
          <FieldLabel>Tags</FieldLabel>
          <TagsInput v-model="formTags" class="py-1.75" data-slot="tags-input-wrapper">
            <TagsInputItem v-for="tag in formTags" :key="tag" :value="tag">
              <TagsInputItemText>{{ tag }}</TagsInputItemText>
              <TagsInputItemDelete />
            </TagsInputItem>
            <TagsInputInput placeholder="monitoring, docker…" />
          </TagsInput>
        </Field>

        <Field>
          <FieldLabel>Scope</FieldLabel>
          <Select v-model="formScope" :disabled="saving">
            <SelectTrigger id="snippet-scope">
              <SelectValue />
            </SelectTrigger>
            <SelectContent>
              <SelectGroup>
                <SelectItem value="global">Global — all hosts</SelectItem>
                <SelectItem value="host">Single host</SelectItem>
                <SelectItem value="hostGroup">Host group</SelectItem>
              </SelectGroup>
            </SelectContent>
          </Select>
        </Field>

        <Field v-if="formScope === 'host'">
          <FieldLabel>Host</FieldLabel>
          <SelectHost v-model="formHostId" placeholder="Select a host…" />
        </Field>

        <Field v-if="formScope === 'hostGroup'">
          <FieldLabel>Host Group</FieldLabel>
          <Select v-model="formHostGroup" :disabled="saving">
            <SelectTrigger id="snippet-host-group">
              <SelectValue placeholder="Select a group…" />
            </SelectTrigger>
            <SelectContent>
              <SelectGroup>
                <SelectItem v-for="groupName in hostGroups" :key="groupName" :value="groupName">
                  {{ groupName }}
                </SelectItem>
              </SelectGroup>
            </SelectContent>
          </Select>
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
          {{ saving ? 'Saving…' : editingId ? 'Save' : 'Create' }}
        </Button>
      </ResponsiveModalFooter>
    </ResponsiveModalContent>
  </ResponsiveModal>
</template>
