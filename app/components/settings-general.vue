<script setup lang="ts">
import { MonitorIcon, MoonIcon, SunIcon } from '@lucide/vue';
import { cn } from '@/lib/utils';

const colorMode = useColorMode();
const { confirmCloseEnabled } = useConfirmClose();

const themeOptions = [
  {
    label: 'Dark',
    icon: MoonIcon,
    value: 'dark' as const,
  },
  {
    label: 'Light',
    icon: SunIcon,
    value: 'light' as const,
  },
  {
    label: 'System',
    icon: MonitorIcon,
    value: 'system' as const,
  },
];
</script>

<template>
  <div class="flex flex-col gap-5">
    <Field>
      <FieldLabel>Theme</FieldLabel>
      <div class="grid grid-cols-3 gap-2">
        <button
          v-for="t in themeOptions"
          :key="t.value"
          type="button"
          :aria-pressed="colorMode.preference === t.value"
          class="flex flex-row items-center justify-center gap-2 rounded-lg border p-1 py-2 transition-colors"
          :class="
            cn(
              'hover:bg-accent/50 hover:text-accent-foreground cursor-pointer',
              colorMode.preference === t.value
                ? 'border-primary/25 bg-accent text-foreground'
                : 'border-border text-muted-foreground',
            )
          "
          @click="colorMode.preference = t.value"
        >
          <component :is="t.icon" class="size-3.5" />
          <span class="text-sm">{{ t.label }}</span>
        </button>
      </div>
      <FieldDescription>Applies to the window chrome and UI controls.</FieldDescription>
    </Field>
    <Field>
      <div class="flex items-center gap-2">
        <Checkbox id="confirm-close" v-model="confirmCloseEnabled" />
        <Label for="confirm-close" class="text-sm cursor-pointer"
          >Warn when closing with active sessions</Label
        >
      </div>
      <FieldDescription
        >Prompts before quitting if any SSH connections are still open.</FieldDescription
      >
    </Field>
  </div>
</template>
