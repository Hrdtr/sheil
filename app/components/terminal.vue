<script lang="ts" setup>
import type { UnlistenFn } from '@tauri-apps/api/event';
import { ArrowDownIcon } from '@lucide/vue';
import { listen } from '@tauri-apps/api/event';
import { FitAddon } from '@xterm/addon-fit';
import { WebLinksAddon } from '@xterm/addon-web-links';
import { WebglAddon } from '@xterm/addon-webgl';
import '@xterm/xterm/css/xterm.css';
import { Terminal } from '@xterm/xterm';

const props = defineProps<{
  sessionId: string | null;
  onTitleChange?: (title: string) => void;
}>();

const containerRef = useTemplateRef('container');

const { appearance, behavior, colorScheme, copyOnSelect } = useTerminalSettings();
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
  cursorInactiveStyle: 'outline',
  scrollback: behavior.value.scrollback,
  allowProposedApi: true,
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

const { macOS } = useKbd();

terminal.attachCustomKeyEventHandler((e) => {
  if (e.type !== 'keydown') return true;
  const mod = macOS.value ? e.metaKey && !e.ctrlKey : e.ctrlKey && e.shiftKey;
  if (mod && e.key.toLowerCase() === 'c') {
    e.preventDefault();
    const selection = terminal.getSelection();
    if (selection) {
      navigator.clipboard.writeText(selection).catch(() => {});
    } else {
      terminal.input('\x03');
    }
    return false;
  }
  if (mod && e.key.toLowerCase() === 'v') {
    e.preventDefault();
    navigator.clipboard
      .readText()
      .then((text) => text && terminal.paste(text))
      .catch(() => {});
    return false;
  }
  return true;
});

terminal.onSelectionChange(() => {
  if (!copyOnSelect.value) return;
  const selection = terminal.getSelection();
  if (selection) navigator.clipboard.writeText(selection).catch(() => {});
});

const scrolledUp = ref(false);

terminal.onScroll(() => {
  const buf = terminal.buffer.active;
  scrolledUp.value = buf.viewportY < buf.baseY;
});

const bellFlash = ref(false);

terminal.onBell(() => {
  bellFlash.value = true;
  setTimeout(() => (bellFlash.value = false), 120);
});

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
  terminal.options.scrollback = behavior.value.scrollback;
});

watch(
  () => [appearance.value.fontSize, appearance.value.fontFamily, appearance.value.lineHeight],
  () => nextTick(() => fitAddon.fit()),
);

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

    registerTerminal(sessionId, { terminal, clear: () => terminal.clear() });

    commands.ssh
      .openChannel(sessionId, terminal.cols, terminal.rows)
      .then(() => commands.ssh.resize(sessionId, terminal.cols, terminal.rows))
      .catch((e) => {
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
    :class="[
      sftpPanelOpen || portForwardingPanelOpen ? 'rounded-xl' : 'rounded-lg',
      bellFlash ? 'bell-flash' : '',
    ]"
    :style="{ backgroundColor: colorScheme.theme.background }"
  >
    <div ref="container" class="absolute top-4 left-4 right-4 bottom-4" />
    <Button
      v-if="scrolledUp"
      variant="secondary"
      size="icon-sm"
      class="absolute bottom-6 right-6 z-10 rounded-full"
      @click="terminal.scrollToBottom()"
    >
      <ArrowDownIcon />
    </Button>
    <AiGhostText
      v-if="aiEnabled && aiInlineEnabled"
      :terminal="terminal"
      :suggestion="ghostSuggestion"
      :visible="ghostVisible"
      @accept="onAcceptSuggestion"
    />
  </div>
</template>
