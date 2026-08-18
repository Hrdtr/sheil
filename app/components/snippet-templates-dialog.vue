<script setup lang="ts">
import { snippetTemplates } from '@/utils/snippet-templates';

const open = defineModel<boolean>('open');

const emit = defineEmits<{
  select: [template: (typeof snippetTemplates)[number]];
}>();

const query = ref('');

const filteredTemplates = computed(() => {
  const q = query.value.trim().toLowerCase();
  if (!q) return snippetTemplates;
  return snippetTemplates.filter((template) =>
    [template.name, template.command, template.description, template.group, ...template.tags]
      .join(' ')
      .toLowerCase()
      .includes(q),
  );
});

const groupedTemplates = computed(() => {
  const groups = new Map<string, typeof snippetTemplates>();
  for (const template of filteredTemplates.value) {
    if (!groups.has(template.group)) groups.set(template.group, []);
    groups.get(template.group)!.push(template);
  }
  return [...groups.entries()].sort(([a], [b]) => a.localeCompare(b));
});

function select(template: (typeof snippetTemplates)[number]) {
  emit('select', template);
  open.value = false;
}

watch(open, (isOpen) => {
  if (isOpen) query.value = '';
});
</script>

<template>
  <ResponsiveModal v-model:open="open" v-slot="{ kind }">
    <ResponsiveModalContent :class="kind === 'dialog' ? 'max-w-md' : ''">
      <ResponsiveModalHeader>
        <ResponsiveModalTitle>Snippet Templates</ResponsiveModalTitle>
        <ResponsiveModalDescription>
          Start from a pre-built snippet for common tasks.
        </ResponsiveModalDescription>
      </ResponsiveModalHeader>

      <div
        class="space-y-4 overflow-y-auto py-1"
        :class="kind === 'dialog' ? 'max-h-[60svh] -mx-6! px-6!' : '-mx-2! px-6!'"
      >
        <div class="w-full overflow-hidden space-y-3">
          <Input v-model="query" placeholder="Search templates…" class="w-full" />

          <div class="w-full overflow-hidden max-h-[50svh] overflow-y-auto pt-2">
            <div
              v-if="groupedTemplates.length === 0"
              class="py-8 text-center text-sm text-muted-foreground"
            >
              No templates match your search.
            </div>
            <div
              v-for="[groupName, templates] in groupedTemplates"
              :key="groupName"
              class="w-full mb-3"
            >
              <p class="mb-1 px-px text-xs font-medium text-muted-foreground">{{ groupName }}</p>
              <div class="w-full flex flex-col">
                <Button
                  v-for="template in templates"
                  :key="template.name"
                  variant="ghost"
                  class="h-auto justify-start px-2 py-1.5 font-normal"
                  @click="select(template)"
                >
                  <div class="flex min-w-0 flex-col gap-0.5 text-left">
                    <span class="truncate text-sm">{{ template.name }}</span>
                    <span class="truncate font-mono text-xs text-muted-foreground">{{
                      template.command
                    }}</span>
                  </div>
                </Button>
              </div>
            </div>
          </div>
        </div>
      </div>
    </ResponsiveModalContent>
  </ResponsiveModal>
</template>
