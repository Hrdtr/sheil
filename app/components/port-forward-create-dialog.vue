<script setup lang="ts">
import { platform } from '@tauri-apps/plugin-os';

const open = defineModel<boolean>('open');

const isDesktop = computed(() => {
  const p = platform();
  return p !== 'android' && p !== 'ios';
});

const { activeSession } = useSessions();
const { startLocal, startRemote, startDynamic, refresh } = usePortForwarding();

// Form state
const forwardType = ref<'local' | 'remote' | 'dynamic'>('local');
const localAddr = ref('127.0.0.1');
const localPort = ref(0);
const remoteHost = ref('');
const remotePort = ref(0);
const targetHost = ref('');
const targetPort = ref(0);

const submitting = ref(false);

function resetForm() {
  forwardType.value = 'local';
  localAddr.value = '127.0.0.1';
  localPort.value = 0;
  remoteHost.value = '';
  remotePort.value = 0;
  targetHost.value = '';
  targetPort.value = 0;
}

watch(open, (isOpen) => {
  if (isOpen) resetForm();
});

async function handleSubmit() {
  const sessionId = activeSession.value?.sshSessionId;
  if (!sessionId) return;

  submitting.value = true;
  try {
    switch (forwardType.value) {
      case 'local':
        await startLocal(
          sessionId,
          localAddr.value,
          localPort.value,
          remoteHost.value,
          remotePort.value,
        );
        break;
      case 'remote':
        await startRemote(
          sessionId,
          'localhost',
          localPort.value,
          targetHost.value,
          targetPort.value,
        );
        break;
      case 'dynamic':
        await startDynamic(sessionId, localAddr.value, localPort.value);
        break;
    }
    await refresh(sessionId);
    open.value = false;
  } catch (e) {
    toast.error(String(e));
  } finally {
    submitting.value = false;
  }
}

const isValid = computed(() => {
  if (!localPort.value || localPort.value < 1 || localPort.value > 65535) return false;
  switch (forwardType.value) {
    case 'local':
      return (
        !!remoteHost.value &&
        !!remotePort.value &&
        remotePort.value >= 1 &&
        remotePort.value <= 65535
      );
    case 'remote':
      return (
        !!targetHost.value &&
        !!targetPort.value &&
        targetPort.value >= 1 &&
        targetPort.value <= 65535
      );
    case 'dynamic':
      return true;
  }
});
</script>

<template>
  <ResponsiveModal v-model:open="open" v-slot="{ kind }">
    <ResponsiveModalContent :class="kind === 'dialog' ? 'max-w-md' : ''">
      <ResponsiveModalHeader>
        <ResponsiveModalTitle>New Port Forward</ResponsiveModalTitle>
        <ResponsiveModalDescription>
          Tunnel a TCP connection through the active SSH session.
        </ResponsiveModalDescription>
      </ResponsiveModalHeader>

      <div
        class="space-y-4 py-1"
        :class="kind === 'dialog' ? 'max-h-[60svh] -mx-6! px-6!' : '-mx-2! px-6!'"
      >
        <!-- Type selector -->
        <Field>
          <FieldLabel>Forward Type</FieldLabel>
          <div class="flex gap-1">
            <Button
              v-for="opt in ['local', 'remote', 'dynamic'] as const"
              :key="opt"
              :variant="forwardType === opt ? 'default' : 'outline'"
              size="sm"
              class="flex-1 text-xs"
              @click="forwardType = opt"
            >
              {{
                {
                  local: 'Local',
                  remote: 'Remote',
                  dynamic: 'SOCKS5',
                }[opt]
              }}
            </Button>
          </div>
        </Field>

        <!-- Local address + port (all types) -->
        <div class="grid grid-cols-3 gap-2">
          <Field class="col-span-2">
            <FieldLabel>
              {{ forwardType === 'remote' ? 'Remote Listen Addr' : 'Local Address' }}
            </FieldLabel>
            <Input
              :model-value="localAddr"
              @update:model-value="localAddr = String($event)"
              placeholder="127.0.0.1"
              :disabled="forwardType === 'remote'"
            />
          </Field>
          <Field>
            <FieldLabel>
              {{ forwardType === 'remote' ? 'Remote Port' : 'Local Port' }}
            </FieldLabel>
            <Input
              :model-value="localPort || ''"
              @update:model-value="localPort = Number($event) || 0"
              type="number"
              placeholder="22"
              min="1"
              max="65535"
            />
          </Field>
        </div>

        <!-- Remote target (local + remote) -->
        <template v-if="forwardType === 'local'">
          <div class="grid grid-cols-3 gap-2">
            <Field class="col-span-2">
              <FieldLabel>Remote Host</FieldLabel>
              <Input
                :model-value="remoteHost"
                @update:model-value="remoteHost = String($event)"
                placeholder="db.internal"
              />
            </Field>
            <Field>
              <FieldLabel>Remote Port</FieldLabel>
              <Input
                :model-value="remotePort || ''"
                @update:model-value="remotePort = Number($event) || 0"
                type="number"
                placeholder="5432"
                min="1"
                max="65535"
              />
            </Field>
          </div>
        </template>

        <template v-if="forwardType === 'remote'">
          <div class="grid grid-cols-3 gap-2">
            <Field class="col-span-2">
              <FieldLabel>Target Host</FieldLabel>
              <Input
                :model-value="targetHost"
                @update:model-value="targetHost = String($event)"
                placeholder="localhost"
              />
            </Field>
            <Field>
              <FieldLabel>Target Port</FieldLabel>
              <Input
                :model-value="targetPort || ''"
                @update:model-value="targetPort = Number($event) || 0"
                type="number"
                placeholder="3000"
                min="1"
                max="65535"
              />
            </Field>
          </div>
        </template>
      </div>

      <ResponsiveModalFooter :class="kind === 'drawer' ? 'flex-col-reverse' : ''">
        <Button variant="outline" @click="open = false">Cancel</Button>
        <Button :disabled="!isValid || submitting" @click="handleSubmit">
          {{ submitting ? 'Starting…' : 'Start Forward' }}
        </Button>
      </ResponsiveModalFooter>
    </ResponsiveModalContent>
  </ResponsiveModal>
</template>
