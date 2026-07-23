<script setup lang="ts">
import type { UnlistenFn } from '@tauri-apps/api/event';
import type { PrimitiveProps } from 'reka-ui';
import type { HTMLAttributes } from 'vue';
import type { ButtonVariants } from './ui/button';
import { MoonIcon, SunIcon, MonitorCogIcon } from '@lucide/vue';
import { getCurrentWindow, getAllWindows } from '@tauri-apps/api/window';

interface Props extends Omit<PrimitiveProps, 'as' | 'asChild'> {
  variant?: ButtonVariants['variant'];
  size?: ButtonVariants['size'];
  class?: HTMLAttributes['class'];
}

const props = defineProps<Props>();

const colorMode = useColorMode();
watch(
  () => colorMode.preference,
  async (value) => {
    const allWindow = await getAllWindows();
    await Promise.all(
      allWindow.map(async (window) =>
        window.setTheme(
          value === 'system' ? await getCurrentWindow().theme() : (value as 'light' | 'dark'),
        ),
      ),
    );
  },
);

const unlistedWindowThemeChangedEvent = ref<UnlistenFn>();
onMounted(async () => {
  unlistedWindowThemeChangedEvent.value = await getCurrentWindow().onThemeChanged(
    ({ payload: theme }) => {
      if (colorMode.preference === theme) return;
      colorMode.preference = theme;
    },
  );
});
onBeforeUnmount(() => {
  unlistedWindowThemeChangedEvent.value?.();
});
</script>

<template>
  <Button
    v-bind="props"
    @click="
      colorMode.preference =
        colorMode.preference === 'light'
          ? 'dark'
          : colorMode.preference === 'dark'
            ? 'system'
            : 'light'
    "
  >
    <SunIcon
      class="absolute size-4 transition-all"
      :class="colorMode.preference === 'light' ? 'rotate-0 scale-100' : 'rotate-90 scale-0'"
    />
    <MoonIcon
      class="size-4 transition-all"
      :class="colorMode.preference === 'dark' ? 'rotate-0 scale-100' : 'rotate-90 scale-0'"
    />
    <MonitorCogIcon
      class="absolute size-4 transition-all"
      :class="colorMode.preference === 'system' ? 'rotate-0 scale-100' : 'rotate-90 scale-0'"
    />
    <span class="sr-only">Toggle theme</span>
  </Button>
</template>
