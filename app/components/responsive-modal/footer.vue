<script setup lang="ts">
import type { HTMLAttributes } from 'vue';
import { useForwardPropsEmits } from 'reka-ui';
import { DialogFooter } from '~/components/ui/dialog';
import { DrawerFooter } from '~/components/ui/drawer';
import { cn } from '~/lib/utils';

const props = withDefaults(
  defineProps<{
    class?: HTMLAttributes['class'];
    showCloseButton?: boolean;
  }>(),
  {
    showCloseButton: false,
  },
);
const delegatedProps = reactiveOmit(props, 'class');
const forwarded = useForwardPropsEmits(delegatedProps);

const dialog = useMediaQuery('(min-width: 640px)');
const Component = computed(
  () => (dialog.value ? DialogFooter : DrawerFooter) as typeof DialogFooter,
);
</script>

<template>
  <component :is="Component" v-bind="forwarded" :class="cn(dialog ? '' : 'pt-4', props.class)">
    <slot />
  </component>
</template>
