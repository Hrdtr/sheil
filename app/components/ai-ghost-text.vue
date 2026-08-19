<script setup lang="ts">
import type { Terminal } from '@xterm/xterm';

interface CellSize {
  width: number;
  height: number;
}

const props = defineProps<{
  terminal: Terminal;
  suggestion: string;
  visible: boolean;
}>();

const emit = defineEmits<{ accept: [] }>();

const left = ref(0);
const top = ref(0);
const cellHeightPx = ref(0);

function getCanvas(): Element | null {
  const el = props.terminal.element;
  if (!el) return null;
  return el.querySelector('.xterm-screen canvas') ?? el.querySelector('canvas');
}

function getCellSize(): CellSize | null {
  const canvas = getCanvas();
  const cols = props.terminal.cols;
  const rows = props.terminal.rows;
  if (canvas && cols > 0 && rows > 0 && canvas.clientWidth > 0 && canvas.clientHeight > 0) {
    return { width: canvas.clientWidth / cols, height: canvas.clientHeight / rows };
  }
  return null;
}

function measureCellWidth(): number {
  const fontSize = props.terminal.options.fontSize ?? 14;
  const fontFamily = props.terminal.options.fontFamily ?? 'monospace';
  const fontWeight = props.terminal.options.fontWeight ?? 'normal';
  const letterSpacing = props.terminal.options.letterSpacing ?? 0;

  if (typeof document === 'undefined') {
    return fontSize * 0.6;
  }

  const ctx = document.createElement('canvas').getContext('2d');
  if (!ctx) {
    return fontSize * 0.6;
  }

  ctx.font = `${fontWeight} ${fontSize}px ${fontFamily}`;
  const measured = ctx.measureText('0').width + letterSpacing;

  return measured > 0 ? measured : fontSize * 0.6;
}

function getOffset(): { x: number; y: number } {
  const el = props.terminal.element;
  const container = el?.closest('.terminal-container');
  const canvas = getCanvas();
  if (container && canvas) {
    const containerRect = container.getBoundingClientRect();
    const canvasRect = canvas.getBoundingClientRect();
    return { x: canvasRect.left - containerRect.left, y: canvasRect.top - containerRect.top };
  }
  return { x: 16, y: 16 };
}

function updatePosition() {
  const buffer = props.terminal.buffer.active;
  const cursorX = buffer.cursorX;
  const cursorY = buffer.cursorY;

  const fontSize = props.terminal.options.fontSize ?? 14;
  const lineHeight = props.terminal.options.lineHeight ?? 1;

  const size = getCellSize() ?? {
    width: measureCellWidth(),
    height: fontSize * lineHeight,
  };
  const offset = getOffset();

  cellHeightPx.value = size.height;
  left.value = offset.x + cursorX * size.width;
  top.value = offset.y + cursorY * size.height;
}

let cursorDispose: { dispose(): void } | undefined;

onMounted(() => {
  cursorDispose = props.terminal.onCursorMove(() => {
    if (props.visible) updatePosition();
  });
  updatePosition();
});

onUnmounted(() => {
  cursorDispose?.dispose();
});

watch([() => props.terminal.options.fontSize, () => props.visible], () => {
  if (props.visible) updatePosition();
});
</script>

<template>
  <div
    v-if="visible && suggestion"
    class="ai-ghost absolute whitespace-pre select-none overflow-visible cursor-pointer"
    :style="{
      left: `${left}px`,
      top: `${top + 1.4}px`,
      fontFamily: terminal.options.fontFamily,
      fontSize: `${terminal.options.fontSize}px`,
      fontWeight: terminal.options.fontWeight,
      letterSpacing: `${terminal.options.letterSpacing ?? 0}px`,
      lineHeight: `${cellHeightPx}px`,
      color: terminal.options.theme?.brightBlack ?? '#6e7681',
      zIndex: 10,
    }"
    @click="emit('accept')"
  >
    {{ suggestion }}
  </div>
</template>

<style scoped>
.ai-ghost {
  opacity: 0.45;
  transition: opacity 140ms ease;
  animation: ai-ghost-in 160ms ease-out;
}

.ai-ghost:hover {
  opacity: 0.85;
}

@keyframes ai-ghost-in {
  from {
    opacity: 0;
  }
}
</style>
