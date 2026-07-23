type Quant = 'Q4_K_M' | 'Q8_0' | 'F16';

interface AiModelFile {
  quant: string;
  filename: string;
  sizeMb: number;
}

interface AiModel {
  id: string;
  name: string;
  repo: string;
  files: AiModelFile[];
}

interface AiSettings {
  enabled: boolean;
  modelId: string;
  quant: Quant;
  inlineCompletionEnabled: boolean;
  commandPaletteEnabled: boolean;
  maxTokens: number;
  temperature: number;
  topP: number;
  contextLines: number;
}

const FALLBACK_MODELS: AiModel[] = [
  {
    id: 'qwen2.5-coder-0.5b-instruct',
    name: 'Qwen 2.5 Coder 0.5B',
    repo: 'Qwen/Qwen2.5-Coder-0.5B-Instruct-GGUF',
    files: [
      { quant: 'Q4_K_M', filename: 'qwen2.5-coder-0.5b-instruct-q4_k_m.gguf', sizeMb: 398 },
      { quant: 'Q8_0', filename: 'qwen2.5-coder-0.5b-instruct-q8_0.gguf', sizeMb: 531 },
    ],
  },
  {
    id: 'smollm2-135m-instruct',
    name: 'SmolLM2 135M',
    repo: 'lmstudio-community/SmolLM2-135M-Instruct-GGUF',
    files: [
      { quant: 'Q4_K_M', filename: 'SmolLM2-135M-Instruct-Q4_K_M.gguf', sizeMb: 95 },
      { quant: 'Q8_0', filename: 'SmolLM2-135M-Instruct-Q8_0.gguf', sizeMb: 145 },
    ],
  },
];

function _useAiSettings() {
  const defaultSettings: AiSettings = {
    enabled: false,
    modelId: 'qwen2.5-coder-0.5b-instruct',
    quant: 'Q4_K_M',
    inlineCompletionEnabled: true,
    commandPaletteEnabled: true,
    maxTokens: 32,
    temperature: 0.2,
    topP: 0.95,
    contextLines: 20,
  };

  const settings = useLocalStorage('ai-settings', () => defaultSettings);
  settings.value = { ...defaultSettings, ...settings.value };

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

  const quant = computed({
    get: () => settings.value.quant,
    set: (value) => {
      settings.value = { ...settings.value, quant: value };
    },
  });

  const inlineCompletionEnabled = computed({
    get: () => settings.value.inlineCompletionEnabled,
    set: (value) => {
      settings.value = { ...settings.value, inlineCompletionEnabled: value };
    },
  });

  const commandPaletteEnabled = computed({
    get: () => settings.value.commandPaletteEnabled,
    set: (value) => {
      settings.value = { ...settings.value, commandPaletteEnabled: value };
    },
  });

  const maxTokensMin = 8;
  const maxTokensMax = 64;

  function clampMaxTokens(value: number): number {
    if (Number.isNaN(value)) return defaultSettings.maxTokens;
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
    if (Number.isNaN(value)) return defaultSettings.temperature;
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
    if (Number.isNaN(value)) return defaultSettings.topP;
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
    if (Number.isNaN(value)) return defaultSettings.contextLines;
    return Math.min(contextLinesMax, Math.max(contextLinesMin, Math.round(value)));
  }

  const contextLines = computed({
    get: () => settings.value.contextLines,
    set: (value) => {
      settings.value = { ...settings.value, contextLines: clampContextLines(value) };
    },
  });

  const models = ref<AiModel[]>(FALLBACK_MODELS);
  const modelsLoadError = ref(false);

  async function loadModels() {
    try {
      const imported = await import('@/assets/ai-models.json').then((m) => m.default);
      if (Array.isArray(imported) && imported.length > 0) {
        models.value = imported as AiModel[];
        modelsLoadError.value = false;
      }
    } catch {
      modelsLoadError.value = true;
    }
  }

  loadModels();

  function selectBestQuant(available: string[]): Quant {
    if (available.includes('Q4_K_M')) return 'Q4_K_M';
    if (available.includes('Q8_0')) return 'Q8_0';
    return 'F16';
  }

  const selectedFile = computed(() => {
    const model = models.value.find((m) => m.id === modelId.value);
    if (!model) return null;
    return model.files.find((f) => f.quant === quant.value) ?? model.files[0] ?? null;
  });

  return {
    settings,
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
    selectBestQuant,
    selectedFile,
  };
}

export const useAiSettings = createSharedComposable(_useAiSettings);
