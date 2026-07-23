<script setup lang="ts">
import type { HTMLAttributes } from 'vue';
import { useForwardPropsEmits } from 'reka-ui';
import { DialogHeader } from '~/components/ui/dialog';
import { DrawerHeader } from '~/components/ui/drawer';

const props = defineProps<{ class?: HTMLAttributes['class'] }>();
const forwarded = useForwardPropsEmits(props);

const dialog = useMediaQuery('(min-width: 640px)');
const Component = computed(
  () => (dialog.value ? DialogHeader : DrawerHeader) as typeof DialogHeader,
);
</script>

<template>
  <component :is="Component" v-bind="forwarded">
    <slot />
  </component>
</template>
