<script setup lang="ts">
import type { DialogRootEmits, DialogRootProps } from 'reka-ui';
import { useForwardPropsEmits } from 'reka-ui';
import { Dialog } from '~/components/ui/dialog';
import { Drawer } from '~/components/ui/drawer';

const props = defineProps<DialogRootProps>();
const emits = defineEmits<DialogRootEmits>();
const forwarded = useForwardPropsEmits(props, emits);

const dialog = useMediaQuery('(min-width: 640px)');
const Component = computed(() => (dialog.value ? Dialog : Drawer) as typeof Dialog);
</script>

<template>
  <component :is="Component" v-slot="slotProps" v-bind="forwarded">
    <slot v-bind="{ ...slotProps, kind: dialog ? 'dialog' : 'drawer' }" />
  </component>
</template>
