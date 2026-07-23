<script setup lang="ts">
import type { DialogTitleProps } from 'reka-ui';
import { useForwardPropsEmits } from 'reka-ui';
import { DialogTitle } from '~/components/ui/dialog';
import { DrawerTitle } from '~/components/ui/drawer';

const props = defineProps<DialogTitleProps>();
const forwarded = useForwardPropsEmits(props);

const dialog = useMediaQuery('(min-width: 640px)');
const Component = computed(() => (dialog.value ? DialogTitle : DrawerTitle) as typeof DialogTitle);
</script>

<template>
  <component :is="Component" v-bind="forwarded">
    <slot />
  </component>
</template>
