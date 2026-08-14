<script setup lang="ts">
import type { ITheme } from '@xterm/xterm';
import { cn } from '@/lib/utils';

const {
  colorSchemes,
  getColorScheme,
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
  scrollbackStep,
  scrollSensitivity,
  scrollSensitivityMin,
  scrollSensitivityMax,
  scrollSensitivityStep,
} = useTerminalSettings();

/** Label of the currently selected font family. */
const selectedFontLabel = computed(
  () => fontFamilyOptions.find((f) => f.value === fontFamily.value)?.label ?? fontFamily.value,
);
/** The 8 base ANSI swatch colors drawn from a scheme's theme. */
function ansiSwatches(theme: ITheme): string[] {
  return [
    theme.black,
    theme.red,
    theme.green,
    theme.yellow,
    theme.blue,
    theme.magenta,
    theme.cyan,
    theme.white,
  ].map((c) => c ?? 'transparent');
}
</script>

<template>
  <div class="flex flex-col gap-5 @container">
    <Field>
      <FieldLabel>Color Scheme</FieldLabel>
      <div class="grid grid-cols-2 @xs:grid-cols-3 gap-2">
        <button
          v-for="scheme in colorSchemes"
          :key="scheme.id"
          type="button"
          :aria-pressed="colorSchemeId === scheme.id"
          :title="scheme.name"
          class="group flex flex-col gap-2 rounded-lg border transition-colors"
          :class="
            cn(
              'cursor-pointer',
              colorSchemeId === scheme.id
                ? 'border-primary/25 bg-accent'
                : 'border-border hover:bg-accent/50',
            )
          "
          @click="colorSchemeId = scheme.id"
        >
          <div
            class="flex h-16 flex-col justify-between rounded-md p-2 font-mono text-[10px] leading-tight transition-transform duration-200"
            :class="colorSchemeId === scheme.id ? 'scale-95' : 'scale-100'"
            :style="{ background: scheme.theme.background, color: scheme.theme.foreground }"
          >
            <span class="truncate text-start leading-none">{{ scheme.name }}</span>
            <span class="truncate text-start leading-none opacity-80 -mt-1">➜ ~ git:(main)</span>
            <div class="flex gap-1">
              <span
                v-for="c in ansiSwatches(scheme.theme)"
                :key="c"
                class="size-2 rounded-[2px] ring-1 ring-black/10"
                :style="{ backgroundColor: c }"
              />
            </div>
          </div>
        </button>
      </div>
      <FieldDescription>Applies to new and existing terminal sessions.</FieldDescription>
    </Field>

    <Field>
      <FieldLabel>Font</FieldLabel>
      <Select v-model="fontFamily">
        <SelectTrigger>
          <SelectValue>
            <span :style="{ fontFamily: fontFamily }">{{ selectedFontLabel }}</span>
          </SelectValue>
        </SelectTrigger>
        <SelectContent>
          <SelectGroup>
            <SelectItem v-for="f in fontFamilyOptions" :key="f.value" :value="f.value">
              <span :style="{ fontFamily: f.value }">{{ f.label }}</span>
            </SelectItem>
          </SelectGroup>
        </SelectContent>
      </Select>
      <FieldDescription>Monospace font family stack.</FieldDescription>
    </Field>

    <Field>
      <div class="flex items-center justify-between">
        <FieldLabel>Font Size</FieldLabel>
        <span class="text-sm tabular-nums text-muted-foreground">{{ fontSize }}px</span>
      </div>
      <Slider
        :model-value="[fontSize]"
        :min="fontSizeMin"
        :max="fontSizeMax"
        :step="1"
        @update:model-value="fontSize = ($event as number[])[0] ?? fontSize"
      />
    </Field>

    <Field>
      <div class="flex items-center justify-between">
        <FieldLabel>Line Height</FieldLabel>
        <span class="text-sm tabular-nums text-muted-foreground">{{ lineHeight.toFixed(1) }}</span>
      </div>
      <Slider
        :model-value="[lineHeight]"
        :min="lineHeightMin"
        :max="lineHeightMax"
        :step="0.1"
        @update:model-value="lineHeight = ($event as number[])[0] ?? lineHeight"
      />
    </Field>

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
      <div class="flex items-center justify-between">
        <FieldLabel>Minimum Contrast Ratio</FieldLabel>
        <span class="text-sm tabular-nums text-muted-foreground">{{ minimumContrastRatio }}</span>
      </div>
      <Slider
        :model-value="[minimumContrastRatio]"
        :min="minimumContrastRatioMin"
        :max="minimumContrastRatioMax"
        :step="1"
        @update:model-value="minimumContrastRatio = ($event as number[])[0] ?? minimumContrastRatio"
      />
      <FieldDescription>1 = off. Higher values force dim text to be more legible.</FieldDescription>
    </Field>

    <Field>
      <div class="flex items-center justify-between">
        <FieldLabel>Scrollback</FieldLabel>
        <span class="text-sm tabular-nums text-muted-foreground">{{ scrollback }} lines</span>
      </div>
      <Slider
        :model-value="[scrollback]"
        :min="scrollbackMin"
        :max="scrollbackMax"
        :step="scrollbackStep"
        @update:model-value="scrollback = ($event as number[])[0] ?? scrollback"
      />
      <FieldDescription>Number of lines kept in scrollback history.</FieldDescription>
    </Field>

    <Field>
      <div class="flex items-center justify-between">
        <FieldLabel>Scroll Sensitivity</FieldLabel>
        <span class="text-sm tabular-nums text-muted-foreground">{{
          scrollSensitivity.toFixed(2)
        }}</span>
      </div>
      <Slider
        :model-value="[scrollSensitivity]"
        :min="scrollSensitivityMin"
        :max="scrollSensitivityMax"
        :step="scrollSensitivityStep"
        @update:model-value="scrollSensitivity = ($event as number[])[0] ?? scrollSensitivity"
      />
      <FieldDescription>Scroll speed multiplier for wheel and trackpad.</FieldDescription>
    </Field>
  </div>
</template>
