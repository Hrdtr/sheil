import { listen } from '@tauri-apps/api/event';

interface EngineState {
  status: 'idle' | 'loading' | 'downloading-model' | 'ready' | 'error';
  modelDownloadProgress: number;
  error: string | null;
}

function _useAiEngine() {
  const { enabled, modelId, quant, maxTokens, temperature, topP, models } = useAiSettings();

  const state = ref<EngineState>({
    status: 'idle',
    modelDownloadProgress: 0,
    error: null,
  });

  let loadedFilename: string | null = null;
  let progressUnlisten: (() => void) | null = null;

  const IDLE_UNLOAD_MS = 60_000;
  let idleTimer: ReturnType<typeof setTimeout> | null = null;

  function resolveFilename(): string | null {
    const model = models.value.find((m) => m.id === modelId.value);
    if (!model) return null;
    const file = model.files.find((f) => f.quant === quant.value);
    return file?.filename ?? model.files[0]?.filename ?? null;
  }

  function resolveRepo(): string | null {
    const model = models.value.find((m) => m.id === modelId.value);
    return model?.repo ?? null;
  }

  async function setupProgressListener() {
    if (progressUnlisten) return;
    progressUnlisten = await listen<{ percent: number }>('ai://download-progress', (event) => {
      state.value = {
        ...state.value,
        status: 'downloading-model',
        modelDownloadProgress: Math.round(event.payload.percent),
      };
    });
  }

  async function loadModel() {
    const filename = resolveFilename();
    if (!filename) {
      state.value = { status: 'error', modelDownloadProgress: 0, error: 'No model file found' };
      return;
    }

    if (loadedFilename === filename) {
      const isLoaded = await commands.ai.isLoaded();
      if (isLoaded) return;
    }

    state.value = { status: 'loading', modelDownloadProgress: 0, error: null };

    try {
      await commands.ai.loadModel(filename);
      loadedFilename = filename;
      state.value = { status: 'ready', modelDownloadProgress: 100, error: null };
    } catch (err) {
      const message = err instanceof Error ? err.message : String(err);
      state.value = { status: 'error', modelDownloadProgress: 0, error: message };
      throw err;
    }
  }

  async function ensureLoaded() {
    const isLoaded = await commands.ai.isLoaded();
    if (!isLoaded) {
      await loadModel();
    }
  }

  function cancelIdleUnload() {
    if (idleTimer) {
      clearTimeout(idleTimer);
      idleTimer = null;
    }
  }

  async function unloadModel() {
    cancelIdleUnload();
    try {
      await commands.ai.unloadModel();
    } catch {
      // Best-effort — the model may already be unloaded.
    }
    loadedFilename = null;
    state.value = { status: 'idle', modelDownloadProgress: 0, error: null };
  }

  function scheduleIdleUnload() {
    cancelIdleUnload();
    idleTimer = setTimeout(() => {
      void unloadModel();
    }, IDLE_UNLOAD_MS);
  }

  async function reloadIfActive() {
    cancelIdleUnload();
    if (loadedFilename !== null || (await commands.ai.isLoaded())) {
      await commands.ai.unloadModel().catch(() => {});
      loadedFilename = null;
    }
    state.value = { status: 'idle', modelDownloadProgress: 0, error: null };

    if (!enabled.value) return;
    const { allCached } = await checkCacheStatus();
    if (allCached) {
      await loadModel().catch(() => {});
      scheduleIdleUnload();
    }
  }

  async function generateCompletion(text: string): Promise<string> {
    cancelIdleUnload();
    await ensureLoaded();
    try {
      return await commands.ai.generate(
        text,
        maxTokens.value,
        temperature.value,
        topP.value,
        'complete',
      );
    } finally {
      scheduleIdleUnload();
    }
  }

  async function generateCommand(instruction: string): Promise<string> {
    cancelIdleUnload();
    await ensureLoaded();
    try {
      return await commands.ai.generate(
        instruction,
        maxTokens.value,
        temperature.value,
        topP.value,
        'command',
      );
    } finally {
      scheduleIdleUnload();
    }
  }

  function cancel() {}

  async function checkCacheStatus(): Promise<{ allCached: boolean; filename: string | null }> {
    const filename = resolveFilename();
    if (!filename) return { allCached: false, filename: null };

    try {
      const downloaded = await commands.ai.listModels();
      const exists = downloaded.some((m) => m.filename === filename);
      return { allCached: exists, filename };
    } catch {
      return { allCached: false, filename };
    }
  }

  async function clearModelCache(): Promise<void> {
    const filename = resolveFilename();
    if (!filename) return;
    await commands.ai.deleteModel(filename);
    if (loadedFilename === filename) {
      loadedFilename = null;
      state.value = { status: 'idle', modelDownloadProgress: 0, error: null };
    }
  }

  async function downloadModel(): Promise<void> {
    const repo = resolveRepo();
    const filename = resolveFilename();
    if (!repo || !filename) {
      throw new Error('Cannot resolve model repo or filename');
    }

    await setupProgressListener();
    state.value = { status: 'downloading-model', modelDownloadProgress: 0, error: null };

    try {
      await commands.ai.downloadModel(repo, filename);
      await commands.ai.loadModel(filename);
      loadedFilename = filename;
      state.value = { status: 'ready', modelDownloadProgress: 100, error: null };
      scheduleIdleUnload();
    } catch (err) {
      const message = err instanceof Error ? err.message : String(err);
      state.value = { status: 'error', modelDownloadProgress: 0, error: message };
      throw err;
    }
  }

  function reloadModel() {
    loadedFilename = null;
    return loadModel();
  }

  watch([modelId, quant], () => {
    void reloadIfActive();
  });

  watch(enabled, (value) => {
    if (!value) void unloadModel();
  });

  return {
    state,
    generateCompletion,
    generateCommand,
    cancel,
    loadModel,
    reloadModel,
    unloadModel,
    checkCacheStatus,
    clearModelCache,
    downloadModel,
    resolveFilename,
  };
}

export const useAiEngine = createSharedComposable(_useAiEngine);
