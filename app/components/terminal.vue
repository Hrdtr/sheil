<script lang="ts" setup>
import type { UnlistenFn } from '@tauri-apps/api/event';
import { listen } from '@tauri-apps/api/event';
import { FitAddon } from '@xterm/addon-fit';
import { WebLinksAddon } from '@xterm/addon-web-links';
import { WebglAddon } from '@xterm/addon-webgl';
import { Terminal } from '@xterm/xterm';
import '@xterm/xterm/css/xterm.css';

const props = defineProps<{
  sessionId: string | null;
  onTitleChange?: (title: string) => void;
}>();

const containerRef = useTemplateRef('container');

const { appearance, colorScheme } = useTerminalSettings();
const { panelOpen: sftpPanelOpen } = useSftp();
const { panelOpen: portForwardingPanelOpen } = usePortForwarding();
const { enabled: aiEnabled, inlineCompletionEnabled: aiInlineEnabled } = useAiSettings();
const {
  register: registerTerminal,
  unregister: unregisterTerminal,
  focus: focusTerminal,
} = useTerminalFocus();

const terminal = new Terminal({
  theme: colorScheme.value.theme,
  fontFamily: appearance.value.fontFamily,
  fontSize: appearance.value.fontSize,
  fontWeight: appearance.value.fontWeight,
  fontWeightBold: appearance.value.fontWeightBold,
  lineHeight: appearance.value.lineHeight,
  cursorStyle: appearance.value.cursorStyle,
  cursorBlink: appearance.value.cursorBlink,
  scrollback: appearance.value.scrollback,
});

const terminalRef = ref(terminal);
const sessionIdRef = toRef(props, 'sessionId');

const { ghostSuggestion, ghostVisible, acceptSuggestion } = useAiCompletion(
  terminalRef,
  sessionIdRef,
);

function onAcceptSuggestion() {
  acceptSuggestion();
  nextTick(() => focusTerminal(props.sessionId));
}

const fitAddon = new FitAddon();
const webLinksAddon = new WebLinksAddon();
terminal.loadAddon(fitAddon);
terminal.loadAddon(webLinksAddon);

try {
  terminal.loadAddon(new WebglAddon());
} catch {
  // WebGL not available — fall back to default canvas renderer
}

// Re-apply appearance whenever the settings store changes. xterm.js
// supports live updates via the `options` setter without re-mounting.
watchEffect(() => {
  terminal.options.theme = colorScheme.value.theme;
  terminal.options.fontFamily = appearance.value.fontFamily;
  terminal.options.fontSize = appearance.value.fontSize;
  terminal.options.fontWeight = appearance.value.fontWeight;
  terminal.options.fontWeightBold = appearance.value.fontWeightBold;
  terminal.options.lineHeight = appearance.value.lineHeight;
  terminal.options.cursorStyle = appearance.value.cursorStyle;
  terminal.options.cursorBlink = appearance.value.cursorBlink;
  terminal.options.scrollback = appearance.value.scrollback;
});

let resizeObserver: ResizeObserver | undefined;

onMounted(() => {
  const el = containerRef.value;
  if (!el) return;

  terminal.open(el);
  fitAddon.fit();

  resizeObserver = new ResizeObserver(() => {
    fitAddon.fit();
  });
  resizeObserver.observe(el);
});

onUnmounted(() => {
  resizeObserver?.disconnect();
  terminal.dispose();
});

// Set up / tear down the SSH PTY channel whenever sessionId changes.
let unlistenOutput: UnlistenFn | undefined;
let unlistenExit: UnlistenFn | undefined;

watch(
  () => props.sessionId,
  (sessionId, _prevSessionId, onCleanup) => {
    if (!sessionId) return;

    registerTerminal(sessionId, terminal);

    const cols = terminal.cols;
    const rows = terminal.rows;

    commands.ssh.openChannel(sessionId, cols, rows).catch((e) => {
      terminal.writeln(`\r\n\x1b[31mPTY error: ${e}\x1b[0m`);
    });

    const titleDispose = terminal.onTitleChange((title) => {
      props.onTitleChange?.(title);
    });

    const dataDispose = terminal.onData((data) => {
      const encoder = new TextEncoder();
      commands.ssh.write(sessionId, encoder.encode(data)).catch(() => {});
    });

    const resizeDispose = terminal.onResize(({ cols, rows }) => {
      commands.ssh.resize(sessionId, cols, rows).catch(() => {});
    });

    listen<{ sessionId: string; data: number[] }>('ssh-output', (event) => {
      if (event.payload.sessionId !== sessionId) return;
      terminal.write(new Uint8Array(event.payload.data));
    }).then((fn) => {
      unlistenOutput = fn;
    });

    listen<{ sessionId: string }>('ssh-exit', (event) => {
      if (event.payload.sessionId !== sessionId) return;
      terminal.writeln('\r\n\x1b[33m[Connection closed]\x1b[0m');
    }).then((fn) => {
      unlistenExit = fn;
    });

    onCleanup(() => {
      titleDispose.dispose();
      dataDispose.dispose();
      resizeDispose.dispose();
      unlistenOutput?.();
      unlistenExit?.();
      unregisterTerminal(sessionId);
      commands.ssh.closeChannel(sessionId).catch(() => {});
    });
  },
  { immediate: true },
);
</script>

<template>
  <div
    class="terminal-container w-full h-full box-border overflow-hidden p-4 relative transition-all"
    :class="sftpPanelOpen || portForwardingPanelOpen ? 'rounded-xl' : 'rounded-lg'"
    :style="{ backgroundColor: colorScheme.theme.background }"
  >
    <div ref="container" class="absolute top-4 left-4 right-4 bottom-4" />
    <AiGhostText
      v-if="aiEnabled && aiInlineEnabled"
      :terminal="terminal"
      :suggestion="ghostSuggestion"
      :visible="ghostVisible"
      @accept="onAcceptSuggestion"
    />
  </div>
</template>
