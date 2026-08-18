<script setup lang="ts">
type Snippet = NonNullable<ReturnType<typeof useSnippets>['snippets']['value']>[number];

const { prepareRun, run, applyVariables } = useSnippets();
const { activeSession } = useSessions();
const { focus: focusTerminal } = useTerminalFocus();

const open = defineModel<boolean>('open');

const emit = defineEmits<{
  ran: [];
}>();

const snippet = ref<Snippet | null>(null);
const submit = ref(true);
const values = ref<Record<string, string>>({});
const running = ref(false);

const confirmDialogOpen = ref(false);
const confirmSnippet = ref<Snippet | null>(null);
const confirmCommand = ref('');

/**
 * Open the run flow for a snippet. Snippets with unresolved variables open
 * the variable form; snippets that are ready to run ask for confirmation
 * first. Returns `true` when the variable form was opened.
 */
function openFor(target: Snippet, shouldSubmit = true): boolean {
  submit.value = shouldSubmit;

  if (!activeSession.value?.sshSessionId || activeSession.value.state !== 'connected') {
    toast.error('No active terminal session');
    return false;
  }

  const context = prepareRun(target);

  if (context.unresolved.length === 0) {
    confirmSnippet.value = target;
    confirmCommand.value = applyVariables(target.command, context.resolved);
    confirmDialogOpen.value = true;
    return false;
  }

  snippet.value = target;
  values.value = Object.fromEntries(context.unresolved.map((name) => [name, '']));
  open.value = true;
  return true;
}

function confirmRun() {
  const target = confirmSnippet.value;
  if (!target) return;
  void execute(target, {});
}

async function execute(target: Snippet, inputValues: Record<string, string>) {
  running.value = true;
  try {
    const sessionId = await run(target, inputValues, submit.value);
    open.value = false;
    confirmDialogOpen.value = false;
    // The dialog restores focus to its trigger on close — re-focus the
    // terminal once the close animation (100ms) has settled.
    setTimeout(() => focusTerminal(sessionId), 150);
    emit('ran');
  } catch (err) {
    // Consistent for both paths: close the dialog and surface the failure
    // as a toast.
    open.value = false;
    confirmDialogOpen.value = false;
    toast.error(err instanceof Error ? err.message : String(err));
  } finally {
    running.value = false;
  }
}

function handleSubmit() {
  if (!snippet.value) return;
  void execute(snippet.value, { ...values.value });
}

watch(open, (isOpen) => {
  if (!isOpen) {
    setTimeout(() => {
      snippet.value = null;
      values.value = {};
    }, 300);
  }
});

watch(confirmDialogOpen, (isOpen) => {
  if (!isOpen) {
    setTimeout(() => {
      confirmSnippet.value = null;
      confirmCommand.value = '';
    }, 300);
  }
});

defineExpose({ openFor });
</script>

<template>
  <ResponsiveModal v-model:open="open" v-slot="{ kind }">
    <ResponsiveModalContent :class="kind === 'dialog' ? 'max-w-md' : ''">
      <ResponsiveModalHeader>
        <ResponsiveModalTitle>{{ submit ? 'Run' : 'Insert' }} Snippet</ResponsiveModalTitle>
        <ResponsiveModalDescription>
          Fill in the variables for
          <span class="text-foreground">{{ snippet?.name }}</span> before
          {{ submit ? 'running' : 'inserting' }}.
        </ResponsiveModalDescription>
      </ResponsiveModalHeader>

      <div
        class="space-y-4 overflow-y-auto py-1"
        :class="kind === 'dialog' ? 'max-h-[60svh] -mx-6! px-6!' : '-mx-2! px-6!'"
      >
        <Field v-for="(_, name, index) in values" :key="name">
          <FieldLabel :for="`snippet-var-${name}`">{{ name }}</FieldLabel>
          <Input
            :id="`snippet-var-${name}`"
            v-model="values[name]"
            :disabled="running"
            :autofocus="index === 0"
          />
        </Field>

        <div
          v-if="snippet"
          class="rounded-lg border bg-muted/50 p-3 font-mono text-xs whitespace-pre-wrap break-all"
        >
          {{ snippet.command }}
        </div>
      </div>

      <ResponsiveModalFooter class="px-0" :class="kind === 'drawer' ? 'flex-col-reverse' : ''">
        <ResponsiveModalClose as-child>
          <Button variant="outline" :disabled="running">Cancel</Button>
        </ResponsiveModalClose>
        <Button :disabled="running" @click="handleSubmit">
          {{ submit ? 'Run' : 'Insert' }}
        </Button>
      </ResponsiveModalFooter>
    </ResponsiveModalContent>
  </ResponsiveModal>

  <ResponsiveModal v-model:open="confirmDialogOpen" v-slot="{ kind }">
    <ResponsiveModalContent :class="kind === 'dialog' ? 'max-w-md' : ''">
      <ResponsiveModalHeader>
        <ResponsiveModalTitle>
          {{ submit ? 'Run snippet?' : 'Insert snippet?' }}
        </ResponsiveModalTitle>
        <ResponsiveModalDescription>
          {{
            submit
              ? `This command from "${confirmSnippet?.name}" will run in the active terminal.`
              : `This command from "${confirmSnippet?.name}" will be inserted into the active terminal.`
          }}
        </ResponsiveModalDescription>
      </ResponsiveModalHeader>

      <div
        class="space-y-4 overflow-y-auto py-1"
        :class="kind === 'dialog' ? 'max-h-[60svh] -mx-6! px-6!' : '-mx-2! px-6!'"
      >
        <div
          class="rounded-lg border bg-muted/50 p-3 font-mono text-xs whitespace-pre-wrap break-all"
        >
          {{ confirmCommand }}
        </div>
      </div>

      <ResponsiveModalFooter class="px-0" :class="kind === 'drawer' ? 'flex-col-reverse' : ''">
        <ResponsiveModalClose as-child>
          <Button variant="outline">Cancel</Button>
        </ResponsiveModalClose>
        <Button @click="confirmRun">
          {{ submit ? 'Run' : 'Insert' }}
        </Button>
      </ResponsiveModalFooter>
    </ResponsiveModalContent>
  </ResponsiveModal>
</template>
