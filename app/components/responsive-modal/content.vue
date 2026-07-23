<script setup lang="ts">
import type { DialogContentEmits, DialogContentProps } from 'reka-ui';
import type { HTMLAttributes } from 'vue';
import { useForwardPropsEmits } from 'reka-ui';
import { DialogContent } from '~/components/ui/dialog';
import { DrawerContent } from '~/components/ui/drawer';
import { cn } from '~/lib/utils';

const props = withDefaults(
  defineProps<
    DialogContentProps & {
      class?: HTMLAttributes['class'];
      showCloseButton?: boolean;
    }
  >(),
  {
    showCloseButton: true,
  },
);
const emits = defineEmits<DialogContentEmits>();
const delegatedProps = reactiveOmit(props, 'class');
const forwarded = useForwardPropsEmits(delegatedProps, emits);

const dialog = useMediaQuery('(min-width: 640px)');
const Component = computed(
  () => (dialog.value ? DialogContent : DrawerContent) as typeof DialogContent,
);
</script>

<template>
  <component
    :is="Component"
    v-bind="forwarded"
    :class="cn('sm:max-w-md', !dialog ? 'px-2 pb-8 *:px-4' : '', props.class)"
  >
    <slot />
  </component>
</template>
