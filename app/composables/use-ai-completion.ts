import type { Terminal } from '@xterm/xterm';

function _useAiCompletion(terminal: Ref<Terminal | null>, sessionId: Ref<string | null>) {
  const { enabled, inlineCompletionEnabled, contextLines } = useAiSettings();
  const { state: engineState, generateCompletion, cancel } = useAiEngine();

  const ghostSuggestion = ref('');
  const ghostVisible = ref(false);
  const isGenerating = ref(false);

  let debounceTimer: ReturnType<typeof setTimeout> | null = null;
  let lastCursorLine = '';

  function dismissSuggestion() {
    ghostVisible.value = false;
    ghostSuggestion.value = '';
  }

  function acceptSuggestion() {
    if (!ghostVisible.value || !ghostSuggestion.value || !sessionId.value) return;
    const encoder = new TextEncoder();
    commands.ssh.write(sessionId.value, encoder.encode(ghostSuggestion.value)).catch(() => {});
    dismissSuggestion();
  }

  function handleKeyboardEvent(event: KeyboardEvent): boolean {
    if (event.type !== 'keydown') return true;
    if (!ghostVisible.value || !ghostSuggestion.value) return true;
    if (event.key === 'Tab' || event.key === 'ArrowRight') {
      event.preventDefault();
      acceptSuggestion();
      return false;
    }
    return true;
  }

  function readCurrentLine(): string {
    const t = terminal.value;
    if (!t) return '';

    const buffer = t.buffer.active;
    if (buffer.type === 'alternate') return '';

    const line = buffer.getLine(buffer.cursorY);
    if (!line) return '';

    return line.translateToString(true, 0, buffer.cursorX);
  }

  function readContextLines(): string {
    const t = terminal.value;
    if (!t) return '';

    const buffer = t.buffer.active;
    const max = contextLines.value;
    const lines: string[] = [];

    const startY = Math.max(0, buffer.cursorY - max);
    for (let y = startY; y < buffer.cursorY; y++) {
      const line = buffer.getLine(y);
      if (line) {
        const text = line.translateToString(true);
        if (text.length > 0) lines.push(text);
      }
    }

    return lines.join('\n');
  }

  async function requestCompletion() {
    if (!enabled.value || !inlineCompletionEnabled.value) return;
    if (!sessionId.value) return;
    if (engineState.value.status !== 'ready' && engineState.value.status !== 'idle') return;

    const currentLine = readCurrentLine();
    if (currentLine.length === 0) return;

    const context = readContextLines();
    lastCursorLine = currentLine;

    isGenerating.value = true;
    dismissSuggestion();

    try {
      const prompt = context
        ? `Recent terminal output:\n${context}\n\nPartial command: ${currentLine}`
        : `Partial command: ${currentLine}`;

      const result = await generateCompletion(prompt);
      let suggestion = result.trim();

      if (currentLine && suggestion.startsWith(currentLine)) {
        suggestion = suggestion.slice(currentLine.length);
      }
      suggestion = suggestion.trimStart();

      if (suggestion.length > 0) {
        ghostSuggestion.value = suggestion;
        ghostVisible.value = true;
      }
    } catch {
      dismissSuggestion();
    } finally {
      isGenerating.value = false;
    }
  }

  function handleUserInput(data: string) {
    if (!enabled.value || !inlineCompletionEnabled.value) return;
    if (!sessionId.value) return;

    const t = terminal.value;
    if (!t) return;
    if (t.buffer.active.type === 'alternate') {
      dismissSuggestion();
      return;
    }

    if (data.length === 1 && data >= ' ') {
      if (debounceTimer) clearTimeout(debounceTimer);
      if (isGenerating.value) cancel();

      if (ghostVisible.value) {
        const nextSuggestionChar = ghostSuggestion.value[0];
        if (nextSuggestionChar && data !== nextSuggestionChar) {
          dismissSuggestion();
        } else if (nextSuggestionChar) {
          ghostSuggestion.value = ghostSuggestion.value.slice(1);
          if (ghostSuggestion.value.length === 0) {
            dismissSuggestion();
          }
        }
      }

      debounceTimer = setTimeout(() => {
        const currentLine = readCurrentLine();
        if (currentLine.length > 0 && currentLine !== lastCursorLine) {
          requestCompletion();
        }
      }, 300);
    } else {
      dismissSuggestion();
    }
  }

  let dataDispose: { dispose(): void } | undefined;
  let cursorDispose: { dispose(): void } | undefined;

  watch(
    [() => terminal.value, () => sessionId.value],
    ([t, sid]) => {
      dataDispose?.dispose();
      cursorDispose?.dispose();

      if (t && sid) {
        dataDispose = t.onData(handleUserInput);
        cursorDispose = t.onCursorMove(() => {
          if (ghostVisible.value) {
            const buffer = t.buffer.active;
            const line = buffer.getLine(buffer.cursorY);
            const currentText = line?.translateToString(true, 0, buffer.cursorX) ?? '';
            if (currentText !== lastCursorLine) {
              dismissSuggestion();
            }
          }
        });
      }
    },
    { immediate: true },
  );

  return {
    ghostSuggestion,
    ghostVisible,
    acceptSuggestion,
    dismissSuggestion,
    handleKeyboardEvent,
  };
}

export const useAiCompletion = (terminal: Ref<Terminal | null>, sessionId: Ref<string | null>) =>
  _useAiCompletion(terminal, sessionId);
