<script setup lang="ts">
const {
  keepaliveInterval,
  keepaliveIntervalMin,
  keepaliveIntervalMax,
  connectTimeout,
  connectTimeoutMin,
  connectTimeoutMax,
  reset,
} = useSshSettings();

async function handleReset() {
  await reset();
  toast.success('SSH settings reset to defaults.');
}
</script>

<template>
  <div class="flex flex-col gap-5">
    <Field>
      <FieldLabel>Keepalive Interval</FieldLabel>
      <Input
        :model-value="keepaliveInterval != null ? String(keepaliveInterval) : '0'"
        type="number"
        :min="keepaliveIntervalMin"
        :max="keepaliveIntervalMax"
        @update:model-value="keepaliveInterval = Number($event) || null"
      />
      <FieldDescription>Seconds between keepalive packets. Set to 0 to disable.</FieldDescription>
    </Field>
    <Field>
      <FieldLabel>Connection Timeout</FieldLabel>
      <Input
        :model-value="connectTimeout != null ? String(connectTimeout) : '30'"
        type="number"
        :min="connectTimeoutMin"
        :max="connectTimeoutMax"
        @update:model-value="connectTimeout = Number($event) || null"
      />
      <FieldDescription
        >Aborts the connection if the server doesn't respond within this time.</FieldDescription
      >
    </Field>

    <div class="flex justify-end">
      <Button variant="outline" size="sm" @click="handleReset">Reset to defaults</Button>
    </div>
  </div>
</template>
