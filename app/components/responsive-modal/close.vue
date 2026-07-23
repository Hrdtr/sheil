<script setup lang="ts">
import type { DialogCloseProps } from 'reka-ui';
import { useForwardPropsEmits } from 'reka-ui';
import { DialogClose } from '~/components/ui/dialog';
import { DrawerClose } from '~/components/ui/drawer';

const props = defineProps<DialogCloseProps>();
const forwarded = useForwardPropsEmits(props);

const dialog = useMediaQuery('(min-width: 640px)');
const Component = computed(() => (dialog.value ? DialogClose : DrawerClose) as typeof DialogClose);
</script>

<template>
  <component :is="Component" v-bind="forwarded">
    <slot />
  </component>
</template>
