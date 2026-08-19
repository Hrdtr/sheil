import { useDebounceFn } from '@vueuse/core';

interface HfModel {
  id: string;
  downloads: number;
  likes: number;
}

interface HfModelFile {
  filename: string;
  sizeBytes: number;
}

interface AiSettings {
  enabled: boolean;
  modelId: string;
  inlineCompletionEnabled: boolean;
  commandGeneratorEnabled: boolean;
  maxTokens: number;
  temperature: number;
  topP: number;
  contextLines: number;
}

interface LegacyModel {
  id: string;
  repo: string;
  filename: string;
}

const LEGACY_MODELS: LegacyModel[] = [
  {
    id: 'qwen2.5-coder-0.5b-instruct',
    repo: 'Qwen/Qwen2.5-Coder-0.5B-Instruct-GGUF',
    filename: 'qwen2.5-coder-0.5b-instruct-q4_k_m.gguf',
  },
  {
    id: 'qwen2.5-coder-1.5b-instruct',
    repo: 'Qwen/Qwen2.5-Coder-1.5B-Instruct-GGUF',
    filename: 'qwen2.5-coder-1.5b-instruct-q4_k_m.gguf',
  },
  {
    id: 'smollm2-135m-instruct',
    repo: 'lmstudio-community/SmolLM2-135M-Instruct-GGUF',
    filename: 'SmolLM2-135M-Instruct-Q4_K_M.gguf',
  },
];

function parseModelId(modelId: string): { repo: string; filename: string } | null {
  const parts = modelId.split('/');
  if (parts.length < 3) return null;
  const repo = `${parts[0]}/${parts[1]}`;
  const filename = parts.slice(2).join('/');
  if (!repo || !filename) return null;
  return { repo, filename };
}

function resolveModelId(modelId: string): { repo: string; filename: string } | null {
  return parseModelId(modelId) ?? LEGACY_MODELS.find((model) => model.id === modelId) ?? null;
}

function smallestFile(files: HfModelFile[]): HfModelFile {
  return files.reduce((min, file) => (file.sizeBytes < min.sizeBytes ? file : min));
}

function pickPreferredFile(files: HfModelFile[]): HfModelFile | null {
  if (files.length === 0) return null;
  const matching = (pattern: RegExp) => files.filter((file) => pattern.test(file.filename));
  const q4KM = matching(/q4_k_m/i);
  if (q4KM.length > 0) return smallestFile(q4KM);
  const q4 = matching(/q4/i);
  if (q4.length > 0) return smallestFile(q4);
  const quantized = matching(/q\d/i);
  if (quantized.length > 0) return smallestFile(quantized);
  return smallestFile(files);
}

function _useAiSettings() {
  const defaults = settingsStore.namespaceDefaults<AiSettings>('ai');
  const settings = useSettings<AiSettings>('ai');

  const enabled = computed({
    get: () => settings.value.enabled,
    set: (value) => {
      settings.value = { ...settings.value, enabled: value };
    },
  });

  const modelId = computed({
    get: () => settings.value.modelId,
    set: (value) => {
      settings.value = { ...settings.value, modelId: value };
    },
  });

  const resolved = computed(() => resolveModelId(modelId.value ?? ''));
  const resolvedRepo = computed(() => resolved.value?.repo ?? null);
  const resolvedFilename = computed(() => resolved.value?.filename ?? null);

  const inlineCompletionEnabled = computed({
    get: () => settings.value.inlineCompletionEnabled,
    set: (value) => {
      settings.value = { ...settings.value, inlineCompletionEnabled: value };
    },
  });

  const commandGeneratorEnabled = computed({
    get: () => settings.value.commandGeneratorEnabled,
    set: (value) => {
      settings.value = { ...settings.value, commandGeneratorEnabled: value };
    },
  });

  const maxTokensMin = 8;
  const maxTokensMax = 64;

  function clampMaxTokens(value: number): number {
    if (Number.isNaN(value)) return defaults.maxTokens;
    return Math.min(maxTokensMax, Math.max(maxTokensMin, Math.round(value)));
  }

  const maxTokens = computed({
    get: () => settings.value.maxTokens,
    set: (value) => {
      settings.value = { ...settings.value, maxTokens: clampMaxTokens(value) };
    },
  });

  const temperatureMin = 0;
  const temperatureMax = 2;

  function clampTemperature(value: number): number {
    if (Number.isNaN(value)) return defaults.temperature;
    return Math.min(temperatureMax, Math.max(temperatureMin, value));
  }

  const temperature = computed({
    get: () => settings.value.temperature,
    set: (value) => {
      settings.value = { ...settings.value, temperature: clampTemperature(value) };
    },
  });

  const topPMin = 0;
  const topPMax = 1;

  function clampTopP(value: number): number {
    if (Number.isNaN(value)) return defaults.topP;
    return Math.min(topPMax, Math.max(topPMin, value));
  }

  const topP = computed({
    get: () => settings.value.topP,
    set: (value) => {
      settings.value = { ...settings.value, topP: clampTopP(value) };
    },
  });

  const contextLinesMin = 5;
  const contextLinesMax = 100;

  function clampContextLines(value: number): number {
    if (Number.isNaN(value)) return defaults.contextLines;
    return Math.min(contextLinesMax, Math.max(contextLinesMin, Math.round(value)));
  }

  const contextLines = computed({
    get: () => settings.value.contextLines,
    set: (value) => {
      settings.value = { ...settings.value, contextLines: clampContextLines(value) };
    },
  });

  const hfModels = ref<HfModel[]>([]);
  const hfModelsLoading = ref(false);
  const hfModelsError = ref(false);
  let searchSeq = 0;

  async function searchHfModels(query = '') {
    const seq = ++searchSeq;
    hfModelsLoading.value = true;
    hfModelsError.value = false;
    try {
      const results = await commands.ai.searchHfModels(query, 50);
      if (seq !== searchSeq) return;
      hfModels.value = results;
    } catch (err) {
      if (seq !== searchSeq) return;
      hfModelsError.value = true;
      console.error('failed to search Hugging Face models:', err);
    } finally {
      if (seq === searchSeq) hfModelsLoading.value = false;
    }
  }

  const debouncedSearchHfModels = useDebounceFn(searchHfModels, 300);

  const selectedRepo = ref<string | null>(resolvedRepo.value);
  const modelFiles = ref<HfModelFile[]>([]);
  const modelFilesLoading = ref(false);
  const modelFilesError = ref(false);
  let filesSeq = 0;

  async function loadModelFiles(repo: string) {
    const seq = ++filesSeq;
    modelFilesLoading.value = true;
    modelFilesError.value = false;
    try {
      const files = await commands.ai.listHfModelFiles(repo);
      if (seq !== filesSeq) return;
      modelFiles.value = files;

      if (files.length === 0) return;
      const current = resolved.value;
      if (current?.repo === repo && files.some((file) => file.filename === current.filename)) {
        if (parseModelId(modelId.value) === null) {
          modelId.value = `${repo}/${current.filename}`;
        }
        return;
      }
      const preferred = pickPreferredFile(files);
      if (preferred) modelId.value = `${repo}/${preferred.filename}`;
    } catch (err) {
      if (seq !== filesSeq) return;
      modelFilesError.value = true;
      console.error('failed to list Hugging Face model files:', err);
    } finally {
      if (seq === filesSeq) modelFilesLoading.value = false;
    }
  }

  function selectRepo(repo: string) {
    selectedRepo.value = repo;
    modelFiles.value = [];
    void loadModelFiles(repo);
  }

  function selectFile(filename: string) {
    const repo = selectedRepo.value ?? resolvedRepo.value;
    if (!repo) return;
    modelId.value = `${repo}/${filename}`;
  }

  watch(resolvedRepo, (repo) => {
    if (repo && repo !== selectedRepo.value) {
      selectedRepo.value = repo;
      void loadModelFiles(repo);
    }
  });

  if (selectedRepo.value) {
    void loadModelFiles(selectedRepo.value);
  }
  void searchHfModels();

  const modelOptions = computed(() => {
    const options = [...hfModels.value];
    const repo = selectedRepo.value;
    if (repo && !options.some((model) => model.id === repo)) {
      options.unshift({ id: repo, downloads: 0, likes: 0 });
    }
    return options;
  });

  const selectedFile = computed(() => {
    const filename = resolvedFilename.value;
    if (!filename) return null;
    return modelFiles.value.find((file) => file.filename === filename) ?? null;
  });

  async function reset(): Promise<void> {
    await settingsStore.resetNamespaces(['ai']);
  }

  return {
    settings,
    enabled,
    modelId,
    resolvedRepo,
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
    hfModels: modelOptions,
    hfModelsLoading,
    hfModelsError,
    searchHfModels,
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
  };
}

export const useAiSettings = createSharedComposable(_useAiSettings);
