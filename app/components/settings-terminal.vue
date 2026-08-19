<script setup lang="ts">
import type { ITheme } from '@xterm/xterm';
import { cn } from '@/lib/utils';

const {
  colorSchemePresets,
  colorScheme,
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
  reset,
} = useTerminalSettings();

async function handleReset() {
  await reset();
  toast.success('Terminal settings reset to defaults.');
}

/** Label of the currently selected font family. */
const selectedFontLabel = computed(
  () =>
    fontFamilyOptions.find((family) => family.value === fontFamily.value)?.label ??
    fontFamily.value,
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
  ].map((color) => color ?? 'transparent');
}

/** Stable key-sorted serialization so theme equality is order-independent. */
function themeSignature(theme: ITheme): string {
  return JSON.stringify(theme, Object.keys(theme).sort());
}

const currentThemeSignature = computed(() => {
  const theme = colorScheme.value;
  return theme ? themeSignature(theme) : '';
});

function isActiveTheme(theme: ITheme): boolean {
  return themeSignature(theme) === currentThemeSignature.value;
}
</script>

<template>
  <div class="flex flex-col gap-5 @container">
    <Field>
      <FieldLabel>Color Scheme</FieldLabel>
      <div class="grid grid-cols-2 @xs:grid-cols-3 gap-2">
        <button
          v-for="scheme in colorSchemePresets"
          :key="scheme.id"
          type="button"
          :aria-pressed="isActiveTheme(scheme.theme)"
          :title="scheme.name"
          class="group flex flex-col gap-2 rounded-lg border transition-colors"
          :class="
            cn(
              'cursor-pointer',
              isActiveTheme(scheme.theme)
                ? 'border-primary/25 bg-accent'
                : 'border-border hover:bg-accent/50',
            )
          "
          @click="colorScheme = scheme.theme"
        >
          <div
            class="flex h-16 flex-col justify-between rounded-md p-2 font-mono text-[10px] leading-tight transition-transform duration-200"
            :class="isActiveTheme(scheme.theme) ? 'scale-95' : 'scale-100'"
            :style="{ background: scheme.theme.background, color: scheme.theme.foreground }"
          >
            <span class="truncate text-start leading-none">{{ scheme.name }}</span>
            <span class="truncate text-start leading-none opacity-80 -mt-1">➜ ~ git:(main)</span>
            <div class="flex gap-1">
              <span
                v-for="color in ansiSwatches(scheme.theme)"
                :key="color"
                class="size-2 rounded-[2px] ring-1 ring-black/10"
                :style="{ backgroundColor: color }"
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
            <SelectItem
              v-for="family in fontFamilyOptions"
              :key="family.value"
              :value="family.value"
            >
              <span :style="{ fontFamily: family.value }">{{ family.label }}</span>
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
            <SelectItem
              v-for="color in cursorStyleOptions"
              :key="color.value"
              :value="color.value"
              >{{ color.label }}</SelectItem
            >
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

    <div class="flex justify-end">
      <Button variant="outline" size="sm" @click="handleReset">Reset to defaults</Button>
    </div>
  </div>
</template>
