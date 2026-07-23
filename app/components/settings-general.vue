<script setup lang="ts">
const colorMode = useColorMode();
const { confirmCloseEnabled } = useConfirmClose();

const themeOptions = [
  { label: 'Light', value: 'light' as const },
  { label: 'Dark', value: 'dark' as const },
  { label: 'System', value: 'system' as const },
];
</script>

<template>
  <div class="flex flex-col gap-5">
    <Field>
      <FieldLabel>Theme</FieldLabel>
      <Select v-model="colorMode.preference">
        <SelectTrigger><SelectValue /></SelectTrigger>
        <SelectContent>
          <SelectGroup>
            <SelectItem v-for="t in themeOptions" :key="t.value" :value="t.value">{{
              t.label
            }}</SelectItem>
          </SelectGroup>
        </SelectContent>
      </Select>
      <FieldDescription>Applies to the window chrome and UI controls.</FieldDescription>
    </Field>
    <Field>
      <div class="flex items-center gap-2">
        <Checkbox id="confirm-close" v-model="confirmCloseEnabled" />
        <Label for="confirm-close" class="text-sm cursor-pointer"
          >Warn when closing with active sessions</Label
        >
      </div>
      <FieldDescription
        >Prompts before quitting if any SSH connections are still open.</FieldDescription
      >
    </Field>
  </div>
</template>
