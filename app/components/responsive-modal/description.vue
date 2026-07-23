<script setup lang="ts">
import type { DialogDescriptionProps } from 'reka-ui';
import { useForwardPropsEmits } from 'reka-ui';
import { DialogDescription } from '~/components/ui/dialog';
import { DrawerDescription } from '~/components/ui/drawer';

const props = defineProps<DialogDescriptionProps>();
const forwarded = useForwardPropsEmits(props);

const dialog = useMediaQuery('(min-width: 640px)');
const Component = computed(
  () => (dialog.value ? DialogDescription : DrawerDescription) as typeof DialogDescription,
);
</script>

<template>
  <component :is="Component" v-bind="forwarded">
    <slot />
  </component>
</template>
