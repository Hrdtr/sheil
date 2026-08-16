<script setup lang="ts">
import { CheckIcon, ChevronsUpDownIcon, MonitorIcon } from '@lucide/vue';

const props = withDefaults(
  defineProps<{
    placeholder?: string;
  }>(),
  { placeholder: 'Select a host…' },
);

const model = defineModel<string | null>({ default: null });

const { hosts, groupedHosts } = useHosts();

const selected = computed(
  () => (hosts.value ?? []).find((host) => host.id === model.value) ?? null,
);

function getDisplayValue(id: string | null): string {
  if (!id) return '';
  return (hosts.value ?? []).find((host) => host.id === id)?.name ?? '';
}
</script>

<template>
  <Combobox v-model="model" class="w-full">
    <ComboboxAnchor as-child>
      <ComboboxTrigger as-child>
        <Button variant="outline" class="w-full justify-between font-normal">
          <span class="flex min-w-0 items-center gap-2">
            <MonitorIcon v-if="selected" class="size-4 shrink-0" />
            <span class="truncate" :class="!selected ? 'text-muted-foreground' : ''">{{
              selected?.name ?? placeholder
            }}</span>
          </span>
          <ChevronsUpDownIcon class="size-4 shrink-0 opacity-50" />
        </Button>
      </ComboboxTrigger>
    </ComboboxAnchor>
    <ComboboxList>
      <ComboboxInput :display-value="getDisplayValue" placeholder="Search hosts…" class="text-sm" />
      <ComboboxViewport class="max-h-64">
        <ComboboxEmpty>No hosts found.</ComboboxEmpty>
        <ComboboxGroup
          v-for="[groupName, groupHosts] in groupedHosts"
          :key="groupName"
          :heading="groupName"
        >
          <ComboboxItem v-for="host in groupHosts" :key="host.id" :value="host.id">
            <MonitorIcon class="size-3.5 shrink-0" />
            <span class="truncate">{{ host.name }}</span>
            <ComboboxItemIndicator>
              <CheckIcon class="size-4" />
            </ComboboxItemIndicator>
          </ComboboxItem>
        </ComboboxGroup>
      </ComboboxViewport>
    </ComboboxList>
  </Combobox>
</template>
