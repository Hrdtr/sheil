<script setup lang="ts">
const {
  enabled,
  modelId,
  quant,
  inlineCompletionEnabled,
  commandPaletteEnabled,
  maxTokens,
  maxTokensMin,
  maxTokensMax,
  temperature,
  temperatureMin,
  temperatureMax,
  topP,
  topPMin,
  topPMax,
  contextLines,
  contextLinesMin,
  contextLinesMax,
  models,
  modelsLoadError,
  selectedFile,
} = useAiSettings();

const { downloadModel, clearModelCache, checkCacheStatus, state: engineState } = useAiEngine();

const isDownloaded = ref(false);
const metadataLoading = ref(false);
const isDownloading = ref(false);
const downloadError = ref<string | null>(null);

const quantLabels: Record<string, string> = {
  Q4_K_M: '4-bit (Q4_K_M)',
  Q8_0: '8-bit (Q8_0)',
  F16: 'Half precision (F16)',
};

function quantLabel(q: string) {
  return quantLabels[q] ?? q;
}

const currentModel = computed(() => models.value.find((m) => m.id === modelId.value));

async function fetchCacheStatus() {
  metadataLoading.value = true;
  try {
    const result = await checkCacheStatus();
    isDownloaded.value = result.allCached;
  } catch {
    isDownloaded.value = false;
  } finally {
    metadataLoading.value = false;
  }
}

async function handleDownload() {
  isDownloading.value = true;
  downloadError.value = null;

  try {
    await downloadModel();
    if (!enabled.value) enabled.value = true;
    toast.success('Model downloaded and ready.');
    await fetchCacheStatus();
  } catch (err) {
    downloadError.value = err instanceof Error ? err.message : String(err);
    toast.error(`Download failed: ${downloadError.value}`);
  } finally {
    isDownloading.value = false;
  }
}

async function handleDelete() {
  try {
    await clearModelCache();
    isDownloaded.value = false;
    toast.success('Model deleted.');
  } catch (err) {
    const message = err instanceof Error ? err.message : String(err);
    toast.error(`Failed to delete model: ${message}`);
  }
}

watch(
  [modelId, quant],
  () => {
    if (enabled.value) fetchCacheStatus();
  },
  { immediate: true },
);

watch(enabled, (value) => {
  if (value) fetchCacheStatus();
});
</script>

<template>
  <div class="flex flex-col gap-5">
    <Field>
      <div class="flex items-center gap-2">
        <Checkbox id="ai-enabled" v-model="enabled" />
        <Label for="ai-enabled" class="text-sm cursor-pointer"> Enable AI completions </Label>
      </div>
      <FieldDescription>
        AI-powered terminal completions run entirely on your device.
      </FieldDescription>
    </Field>

    <template v-if="enabled">
      <Field>
        <FieldLabel>Model</FieldLabel>
        <Select v-model="modelId">
          <SelectTrigger><SelectValue /></SelectTrigger>
          <SelectContent>
            <SelectGroup>
              <SelectItem v-for="m in models" :key="m.id" :value="m.id">
                {{ m.name }}
              </SelectItem>
            </SelectGroup>
          </SelectContent>
        </Select>
        <FieldDescription v-if="modelsLoadError"> Using default model list. </FieldDescription>
      </Field>

      <Field>
        <FieldLabel>Quantization</FieldLabel>
        <Select v-model="quant">
          <SelectTrigger><SelectValue /></SelectTrigger>
          <SelectContent>
            <SelectGroup>
              <SelectItem v-for="f in currentModel?.files ?? []" :key="f.quant" :value="f.quant">
                {{ quantLabel(f.quant) }}
              </SelectItem>
            </SelectGroup>
          </SelectContent>
        </Select>
        <FieldDescription>
          Smaller quantizations are faster and use less memory but are less precise.
        </FieldDescription>
      </Field>

      <div v-if="selectedFile" class="space-y-2 text-sm">
        <div v-if="metadataLoading" class="flex items-center gap-2 text-muted-foreground">
          <Skeleton class="h-3 w-3 rounded-full" />
          <span>Checking model status…</span>
        </div>

        <template v-else>
          <div class="flex items-center gap-2">
            <span
              class="inline-block w-2 h-2 rounded-full"
              :class="isDownloaded ? 'bg-green-500' : 'bg-muted-foreground'"
            />
            <span>{{ isDownloaded ? 'Downloaded' : 'Not downloaded' }}</span>
            <span class="text-muted-foreground">({{ selectedFile.sizeMb }} MB)</span>
          </div>

          <div class="text-xs text-muted-foreground font-mono">
            {{ selectedFile.filename }}
          </div>

          <div v-if="!isDownloaded">
            <Button
              variant="link"
              size="sm"
              :disabled="isDownloading"
              class="px-0"
              @click="handleDownload"
            >
              {{
                isDownloading
                  ? engineState.modelDownloadProgress > 0
                    ? `Downloading… ${engineState.modelDownloadProgress}%`
                    : 'Downloading…'
                  : 'Download'
              }}
            </Button>
            <div v-if="downloadError" class="text-xs text-destructive mt-1">
              {{ downloadError }}
            </div>
          </div>

          <div v-if="isDownloaded">
            <Button variant="link" size="sm" class="text-destructive px-0" @click="handleDelete">
              Delete model
            </Button>
          </div>
        </template>
      </div>

      <Field>
        <div class="flex items-center gap-2">
          <Checkbox id="ai-inline" v-model="inlineCompletionEnabled" />
          <Label for="ai-inline" class="text-sm cursor-pointer"> Inline ghost text </Label>
        </div>
        <FieldDescription>
          Shows dimmed completion text after the cursor as you type commands.
        </FieldDescription>
      </Field>

      <Field>
        <div class="flex items-center gap-2">
          <Checkbox id="ai-palette" v-model="commandPaletteEnabled" />
          <Label for="ai-palette" class="text-sm cursor-pointer"> Command palette (Cmd+I) </Label>
        </div>
        <FieldDescription>
          Opens a dialog to generate shell commands from natural language.
        </FieldDescription>
      </Field>

      <Collapsible>
        <CollapsibleTrigger class="flex items-center gap-1 text-sm font-medium hover:underline">
          Advanced
        </CollapsibleTrigger>
        <CollapsibleContent class="mt-3 space-y-4">
          <div class="grid grid-cols-2 gap-3">
            <Field>
              <FieldLabel>Max Tokens</FieldLabel>
              <Input
                :model-value="String(maxTokens)"
                type="number"
                :min="maxTokensMin"
                :max="maxTokensMax"
                @update:model-value="maxTokens = Number($event)"
              />
              <FieldDescription>Max tokens to generate (8–64).</FieldDescription>
            </Field>
            <Field>
              <FieldLabel>Temperature</FieldLabel>
              <Input
                :model-value="String(temperature)"
                type="number"
                :min="temperatureMin"
                :max="temperatureMax"
                step="0.1"
                @update:model-value="temperature = Number($event)"
              />
              <FieldDescription>Creativity (0–2). Lower = more deterministic.</FieldDescription>
            </Field>
          </div>
          <div class="grid grid-cols-2 gap-3">
            <Field>
              <FieldLabel>Top P</FieldLabel>
              <Input
                :model-value="String(topP)"
                type="number"
                :min="topPMin"
                :max="topPMax"
                step="0.05"
                @update:model-value="topP = Number($event)"
              />
              <FieldDescription>Nucleus sampling threshold (0–1).</FieldDescription>
            </Field>
            <Field>
              <FieldLabel>Context Lines</FieldLabel>
              <Input
                :model-value="String(contextLines)"
                type="number"
                :min="contextLinesMin"
                :max="contextLinesMax"
                @update:model-value="contextLines = Number($event)"
              />
              <FieldDescription>Terminal history lines sent to model (5–100).</FieldDescription>
            </Field>
          </div>
        </CollapsibleContent>
      </Collapsible>
    </template>
  </div>
</template>
