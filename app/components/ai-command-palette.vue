<script setup lang="ts">
const open = defineModel<boolean>('open');
const { generateCommand } = useAiEngine();
const { activeSession } = useSessions();
const { focus: focusTerminal } = useTerminalFocus();

const input = ref('');
const generatedCommand = ref('');
const status = ref<'idle' | 'loading' | 'result' | 'error'>('idle');
const errorMessage = ref('');
const inputRef = useTemplateRef('promptInput');

const submitDisabled = computed(() => !input.value.trim() || status.value === 'loading');

async function generate() {
  const prompt = input.value.trim();
  if (!prompt) return;

  status.value = 'loading';
  errorMessage.value = '';
  generatedCommand.value = '';

  try {
    const result = await generateCommand(prompt);
    generatedCommand.value = result.trim();
    status.value = 'result';
  } catch (err) {
    errorMessage.value = err instanceof Error ? err.message : String(err);
    status.value = 'error';
  }
}

function cancel() {
  if (status.value === 'result') {
    status.value = 'idle';
    generatedCommand.value = '';
    errorMessage.value = '';
    nextTick(() => {
      const el = inputRef.value?.$el;
      const target = el instanceof HTMLInputElement ? el : el?.querySelector('input');
      target?.focus();
    });
  } else {
    open.value = false;
  }
}

function insert() {
  const sessionId = activeSession.value?.sshSessionId;
  if (!sessionId || !generatedCommand.value) return;

  const encoder = new TextEncoder();
  commands.ssh.write(sessionId, encoder.encode(generatedCommand.value)).catch(() => {});
  open.value = false;
  nextTick(() => focusTerminal(sessionId));
}

function onKeydown(event: KeyboardEvent) {
  if (event.key === 'Enter' && !event.shiftKey) {
    event.preventDefault();
    if (status.value === 'result') {
      insert();
    } else {
      generate();
    }
  }
}

watch(open, (isOpen) => {
  if (isOpen) {
    input.value = '';
    generatedCommand.value = '';
    status.value = 'idle';
    errorMessage.value = '';
  }
});
</script>

<template>
  <Dialog v-model:open="open">
    <DialogContent class="max-w-lg">
      <DialogHeader>
        <DialogTitle>AI Command Generator</DialogTitle>
        <DialogDescription>
          Describe what you want to do and an AI-generated shell command will be inserted at the
          cursor.
        </DialogDescription>
      </DialogHeader>

      <div class="space-y-4 py-1">
        <Field v-if="status !== 'result'">
          <Input
            ref="promptInput"
            v-model="input"
            placeholder="e.g. find all files larger than 1GB"
            :disabled="status === 'loading'"
            @keydown="onKeydown"
          />
        </Field>

        <div
          v-if="status === 'loading'"
          class="flex items-center gap-2 text-sm text-muted-foreground"
        >
          <Skeleton class="h-4 w-4 rounded-full" />
          <span>Generating command…</span>
        </div>

        <div
          v-if="status === 'result'"
          class="rounded-lg border bg-muted/50 p-3 font-mono text-sm whitespace-pre-wrap break-all"
        >
          {{ generatedCommand }}
        </div>

        <div v-if="status === 'error'" class="text-sm text-destructive">
          {{ errorMessage }}
        </div>
      </div>

      <DialogFooter>
        <Button variant="outline" @click="cancel">
          {{ status === 'result' ? 'Back' : 'Cancel' }}
        </Button>
        <Button
          v-if="status === 'idle' || status === 'error'"
          :disabled="submitDisabled"
          @click="generate"
        >
          Generate
        </Button>
        <Button v-if="status === 'result'" @click="insert">Insert</Button>
      </DialogFooter>
    </DialogContent>
  </Dialog>
</template>
