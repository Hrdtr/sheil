<script setup lang="ts">
import type { DialogTriggerProps } from 'reka-ui';
import { useForwardPropsEmits } from 'reka-ui';
import { DialogTrigger } from '~/components/ui/dialog';
import { DrawerTrigger } from '~/components/ui/drawer';

const props = defineProps<DialogTriggerProps>();
const forwarded = useForwardPropsEmits(props);

const dialog = useMediaQuery('(min-width: 640px)');
const Component = computed(
  () => (dialog.value ? DialogTrigger : DrawerTrigger) as typeof DialogTrigger,
);
</script>

<template>
  <component :is="Component" v-bind="forwarded">
    <slot />
  </component>
</template>
