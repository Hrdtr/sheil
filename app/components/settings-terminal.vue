<script setup lang="ts">
const {
  colorSchemeId,
  fontSize,
  fontSizeMin,
  fontSizeMax,
  fontFamily,
  fontFamilyOptions,
  lineHeight,
  lineHeightMin,
  lineHeightMax,
  cursorStyle,
  cursorStyleOptions,
  cursorBlink,
  minimumContrastRatio,
  minimumContrastRatioMin,
  minimumContrastRatioMax,
  copyOnSelect,
  scrollback,
  scrollbackMin,
  scrollbackMax,
} = useTerminalSettings();

const colorSchemes = [
  { id: 'catppuccin-mocha' as const, name: 'Catppuccin Mocha' },
  { id: 'catppuccin-latte' as const, name: 'Catppuccin Latte' },
  { id: 'dracula' as const, name: 'Dracula' },
  { id: 'nord' as const, name: 'Nord' },
  { id: 'solarized-dark' as const, name: 'Solarized Dark' },
  { id: 'solarized-light' as const, name: 'Solarized Light' },
  { id: 'github-dark' as const, name: 'GitHub Dark' },
  { id: 'one-dark' as const, name: 'One Dark' },
  { id: 'tokyo-night' as const, name: 'Tokyo Night' },
];
</script>

<template>
  <div class="flex flex-col gap-5">
    <Field>
      <FieldLabel>Color Scheme</FieldLabel>
      <Select v-model="colorSchemeId">
        <SelectTrigger><SelectValue /></SelectTrigger>
        <SelectContent>
          <SelectGroup>
            <SelectItem v-for="s in colorSchemes" :key="s.id" :value="s.id">{{
              s.name
            }}</SelectItem>
          </SelectGroup>
        </SelectContent>
      </Select>
      <FieldDescription>Applies to new and existing terminal sessions.</FieldDescription>
    </Field>
    <Field>
      <FieldLabel>Font</FieldLabel>
      <Select v-model="fontFamily">
        <SelectTrigger><SelectValue /></SelectTrigger>
        <SelectContent>
          <SelectGroup>
            <SelectItem v-for="f in fontFamilyOptions" :key="f.value" :value="f.value">{{
              f.label
            }}</SelectItem>
          </SelectGroup>
        </SelectContent>
      </Select>
      <FieldDescription>Monospace font family stack.</FieldDescription>
    </Field>
    <div class="grid grid-cols-2 gap-3">
      <Field>
        <FieldLabel>Font Size</FieldLabel>
        <Input
          :model-value="String(fontSize)"
          type="number"
          :min="fontSizeMin"
          :max="fontSizeMax"
          @update:model-value="fontSize = Number($event)"
        />
        <FieldDescription>Size in pixels.</FieldDescription>
      </Field>
      <Field>
        <FieldLabel>Line Height</FieldLabel>
        <Input
          :model-value="String(lineHeight)"
          type="number"
          :min="lineHeightMin"
          :max="lineHeightMax"
          step="0.1"
          @update:model-value="lineHeight = Number($event)"
        />
        <FieldDescription>Line spacing multiplier.</FieldDescription>
      </Field>
    </div>
    <Field>
      <FieldLabel>Cursor</FieldLabel>
      <Select v-model="cursorStyle">
        <SelectTrigger><SelectValue /></SelectTrigger>
        <SelectContent>
          <SelectGroup>
            <SelectItem v-for="c in cursorStyleOptions" :key="c.value" :value="c.value">{{
              c.label
            }}</SelectItem>
          </SelectGroup>
        </SelectContent>
      </Select>
      <FieldDescription>Cursor shape.</FieldDescription>
    </Field>
    <Field class="justify-end">
      <div class="flex items-center gap-2">
        <Checkbox id="cursor-blink" v-model="cursorBlink" />
        <Label for="cursor-blink" class="text-sm cursor-pointer">Cursor Blink</Label>
      </div>
      <FieldDescription>Flashes the cursor to make it easier to find.</FieldDescription>
    </Field>
    <Field class="justify-end">
      <div class="flex items-center gap-2">
        <Checkbox id="copy-on-select" v-model="copyOnSelect" />
        <Label for="copy-on-select" class="text-sm cursor-pointer">Copy on Select</Label>
      </div>
      <FieldDescription>Copies selected text to the clipboard on release.</FieldDescription>
    </Field>
    <Field>
      <FieldLabel>Minimum Contrast Ratio</FieldLabel>
      <Input
        :model-value="String(minimumContrastRatio)"
        type="number"
        :min="minimumContrastRatioMin"
        :max="minimumContrastRatioMax"
        step="0.5"
        @update:model-value="minimumContrastRatio = Number($event)"
      />
      <FieldDescription>1 = off. Higher values force dim text to be more legible.</FieldDescription>
    </Field>
    <Field>
      <FieldLabel>Scrollback</FieldLabel>
      <Input
        :model-value="String(scrollback)"
        type="number"
        :min="scrollbackMin"
        :max="scrollbackMax"
        step="500"
        @update:model-value="scrollback = Number($event)"
      />
      <FieldDescription>Number of lines kept in scrollback history.</FieldDescription>
    </Field>
  </div>
</template>
