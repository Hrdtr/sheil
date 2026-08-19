<script setup lang="ts">
import { CheckIcon, ChevronsUpDownIcon } from '@lucide/vue';

const {
  enabled,
  modelId,
  resolvedFilename,
  inlineCompletionEnabled,
  commandGeneratorEnabled,
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
  hfModels,
  hfModelsLoading,
  hfModelsError,
  debouncedSearchHfModels,
  selectedRepo,
  selectRepo,
  modelFiles,
  modelFilesLoading,
  modelFilesError,
  loadModelFiles,
  selectFile,
  selectedFile,
  reset,
} = useAiSettings();

async function handleReset() {
  await reset();
  toast.success('AI settings reset to defaults.');
}

const { downloadModel, clearModelCache, checkCacheStatus, state: engineState } = useAiEngine();

const isDownloaded = ref(false);
const metadataLoading = ref(false);
const isDownloading = ref(false);
const downloadError = ref<string | null>(null);
const searchTerm = ref('');

function quantLabel(filename: string): string {
  const base = filename.replace(/\.gguf$/i, '').replace(/^.*\//, '');
  const matches = [...base.matchAll(/iq\d+[a-z0-9_]*|q\d+(?:_[a-z0-9]+)*|fp16|bf16|f16|f32/gi)];
  const last = matches[matches.length - 1];
  return last ? last[0].toUpperCase() : base;
}

function formatBytes(bytes: number): string {
  if (bytes >= 1024 ** 3) return `${(bytes / 1024 ** 3).toFixed(1)} GB`;
  return `${Math.max(1, Math.round(bytes / 1024 ** 2))} MB`;
}

function formatCount(count: number): string {
  if (count >= 1_000_000) return `${(count / 1_000_000).toFixed(1)}M`;
  if (count >= 1_000) return `${Math.round(count / 1_000)}k`;
  return String(count);
}

function handleSearchTerm(value: string) {
  if (!value) return;
  searchTerm.value = value;
  void debouncedSearchHfModels(value);
}

function handleRepoSelect(value: unknown) {
  if (typeof value !== 'string' || !value) return;
  searchTerm.value = '';
  selectRepo(value);
  if (enabled.value) fetchCacheStatus();
}

function handleFileSelect(value: unknown) {
  if (typeof value !== 'string' || !value) return;
  selectFile(value);
}

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
  modelId,
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
        <FieldLabel>Model Repository</FieldLabel>
        <Combobox :model-value="selectedRepo" ignore-filter @update:model-value="handleRepoSelect">
          <ComboboxAnchor as-child>
            <ComboboxTrigger as-child>
              <Button variant="outline" class="w-full justify-between font-normal">
                <span class="truncate" :class="!selectedRepo ? 'text-muted-foreground' : ''">
                  {{ selectedRepo ?? 'Search Hugging Face models…' }}
                </span>
                <ChevronsUpDownIcon class="size-4 shrink-0 opacity-50" />
              </Button>
            </ComboboxTrigger>
          </ComboboxAnchor>
          <ComboboxList>
            <ComboboxInput
              :model-value="searchTerm"
              :display-value="() => ''"
              placeholder="Search GGUF model repositories…"
              class="text-sm"
              @update:model-value="handleSearchTerm"
            />
            <ComboboxViewport class="max-h-64">
              <div
                v-if="hfModelsLoading"
                class="flex items-center gap-2 px-2 py-4 text-sm text-muted-foreground"
              >
                <Skeleton class="h-3 w-3 rounded-full" />
                <span>Searching Hugging Face…</span>
              </div>
              <template v-else>
                <ComboboxEmpty>
                  {{
                    hfModelsError ? 'Failed to load models from Hugging Face.' : 'No models found.'
                  }}
                </ComboboxEmpty>
                <ComboboxGroup>
                  <ComboboxItem v-for="model in hfModels" :key="model.id" :value="model.id">
                    <div class="flex flex-col items-start">
                      <span class="truncate">{{ model.id }}</span>
                      <span class="shrink-0 text-xs text-muted-foreground">
                        {{ model.id === selectedRepo ? 'Selected' : '' }}
                        <span v-if="model.downloads > 0">
                          {{ model.id === selectedRepo ? '· ' : ''
                          }}{{ formatCount(model.downloads) }} downloads
                        </span>
                      </span>
                    </div>
                    <ComboboxItemIndicator>
                      <CheckIcon class="size-4" />
                    </ComboboxItemIndicator>
                  </ComboboxItem>
                </ComboboxGroup>
              </template>
            </ComboboxViewport>
          </ComboboxList>
        </Combobox>
        <FieldDescription>GGUF model repositories from huggingface.co.</FieldDescription>
      </Field>

      <Field>
        <FieldLabel>Model File</FieldLabel>
        <div v-if="modelFilesLoading" class="flex items-center gap-2 py-1.5">
          <Skeleton class="h-3 w-3 rounded-full" />
          <span class="text-sm text-muted-foreground">Loading model files…</span>
        </div>
        <div v-else-if="modelFilesError" class="text-sm">
          <span class="text-destructive">Failed to load model files.</span>
          <Button
            v-if="selectedRepo"
            variant="link"
            size="sm"
            class="px-1"
            @click="loadModelFiles(selectedRepo)"
          >
            Retry
          </Button>
        </div>
        <Select v-else :model-value="resolvedFilename" @update:model-value="handleFileSelect">
          <SelectTrigger>
            <SelectValue
              :aria-label="resolvedFilename || 'Select a model file…'"
              :class="!resolvedFilename ? 'text-muted-foreground' : ''"
            >
              {{ resolvedFilename || 'Select a model file…' }}
            </SelectValue>
          </SelectTrigger>
          <SelectContent>
            <SelectGroup>
              <SelectItem v-for="file in modelFiles" :key="file.filename" :value="file.filename">
                <div class="flex flex-col items-start">
                  <span class="truncate">{{ file.filename }}</span>
                  <span class="shrink-0 text-xs text-muted-foreground">
                    {{ quantLabel(file.filename) }} · {{ formatBytes(file.sizeBytes) }}
                  </span>
                </div>
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
            <span class="text-muted-foreground">({{ formatBytes(selectedFile.sizeBytes) }})</span>
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
          <Checkbox id="ai-inline-completion" v-model="inlineCompletionEnabled" />
          <Label for="ai-inline-completion" class="text-sm cursor-pointer">
            Inline ghost text
          </Label>
        </div>
        <FieldDescription>
          Shows dimmed completion text after the cursor as you type commands.
        </FieldDescription>
      </Field>

      <Field>
        <div class="flex items-center gap-2">
          <Checkbox id="ai-command-generator" v-model="commandGeneratorEnabled" />
          <Label for="ai-command-generator" class="text-sm cursor-pointer">
            Command Generator (Cmd+I)
          </Label>
        </div>
        <FieldDescription>
          Opens a dialog to generate shell commands from natural language.
        </FieldDescription>
      </Field>

      <Collapsible v-slot="{ open }">
        <CollapsibleTrigger class="flex items-center gap-1 text-sm font-medium hover:underline">
          {{ open ? 'Hide' : 'Show' }} advanced settings
        </CollapsibleTrigger>
        <CollapsibleContent class="mt-4 space-y-4">
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

    <div class="flex justify-end">
      <Button variant="outline" size="sm" @click="handleReset">Reset to defaults</Button>
    </div>
  </div>
</template>
