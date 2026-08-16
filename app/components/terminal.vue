<script lang="ts" setup>
import type { UnlistenFn } from '@tauri-apps/api/event';
import { ArrowDownIcon, ChevronDownIcon, ChevronUpIcon, XIcon } from '@lucide/vue';
import { listen } from '@tauri-apps/api/event';
import { readText, writeText } from '@tauri-apps/plugin-clipboard-manager';
import { FitAddon } from '@xterm/addon-fit';
import { SearchAddon } from '@xterm/addon-search';
import { Unicode11Addon } from '@xterm/addon-unicode11';
import { WebLinksAddon } from '@xterm/addon-web-links';
import { WebglAddon } from '@xterm/addon-webgl';
import { Terminal } from '@xterm/xterm';
import '@xterm/xterm/css/xterm.css';

const props = defineProps<{
  sessionId: string | null;
  onTitleChange?: (title: string) => void;
}>();

const containerRef = useTemplateRef('container');

const { appearance, behavior, colorScheme, copyOnSelect, minimumContrastRatio, scrollSensitivity } =
  useTerminalSettings();
const { panelOpen: sftpPanelOpen } = useSftp();
const { panelOpen: portForwardingPanelOpen } = usePortForwarding();
const { enabled: aiEnabled, inlineCompletionEnabled: aiInlineEnabled } = useAiSettings();
const {
  register: registerTerminal,
  unregister: unregisterTerminal,
  focus: focusTerminal,
} = useTerminalFocus();

const terminal = new Terminal({
  theme: colorScheme.value,
  fontFamily: appearance.value.fontFamily,
  fontSize: appearance.value.fontSize,
  fontWeight: appearance.value.fontWeight,
  fontWeightBold: appearance.value.fontWeightBold,
  lineHeight: appearance.value.lineHeight,
  cursorStyle: appearance.value.cursorStyle,
  cursorBlink: appearance.value.cursorBlink,
  cursorInactiveStyle: 'outline',
  minimumContrastRatio: appearance.value.minimumContrastRatio,
  scrollback: behavior.value.scrollback,
  scrollSensitivity: behavior.value.scrollSensitivity,
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
const searchAddon = new SearchAddon();
const unicode11 = new Unicode11Addon();
terminal.loadAddon(fitAddon);
terminal.loadAddon(webLinksAddon);
terminal.loadAddon(searchAddon);
terminal.loadAddon(unicode11);
terminal.unicode.activeVersion = '11';

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
      writeText(selection).catch(() => {});
    } else {
      terminal.input('\x03');
    }
    return false;
  }
  if (mod && e.key.toLowerCase() === 'v') {
    e.preventDefault();
    readText()
      .then((text) => text && terminal.paste(text))
      .catch(() => {});
    return false;
  }
  return true;
});

terminal.onSelectionChange(() => {
  if (!copyOnSelect.value) return;
  const selection = terminal.getSelection();
  if (selection) writeText(selection).catch(() => {});
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

const searchOpen = ref(false);
const searchTerm = ref('');
const hasMatches = ref(false);
const searchInputRef = useTemplateRef('searchInput');

function openSearch() {
  if (searchOpen.value) {
    closeSearch();
    return;
  }
  searchOpen.value = true;
  nextTick(() => (searchInputRef.value?.$el as HTMLInputElement)?.focus());
}

function closeSearch() {
  searchOpen.value = false;
  searchTerm.value = '';
  hasMatches.value = false;
  searchAddon.clearDecorations();
  focusTerminal(props.sessionId);
}

function findNext() {
  if (!searchTerm.value) return;
  hasMatches.value = searchAddon.findNext(searchTerm.value, {
    decorations: {
      matchBackground: colorScheme.value.selectionBackground ?? '#585b70',
      activeMatchBackground: colorScheme.value.cursor ?? '#f5e0dc',
      matchOverviewRuler: colorScheme.value.cursor ?? '#f5e0dc',
      activeMatchColorOverviewRuler: colorScheme.value.cursor ?? '#f5e0dc',
    },
  });
}

function findPrev() {
  if (!searchTerm.value) return;
  hasMatches.value = searchAddon.findPrevious(searchTerm.value, {
    decorations: {
      matchBackground: colorScheme.value.selectionBackground ?? '#585b70',
      activeMatchBackground: colorScheme.value.cursor ?? '#f5e0dc',
      matchOverviewRuler: colorScheme.value.cursor ?? '#f5e0dc',
      activeMatchColorOverviewRuler: colorScheme.value.cursor ?? '#f5e0dc',
    },
  });
}

// Re-apply appearance whenever the settings store changes. xterm.js
// supports live updates via the `options` setter without re-mounting.
watchEffect(() => {
  terminal.options.theme = colorScheme.value;
  terminal.options.fontFamily = appearance.value.fontFamily;
  terminal.options.fontSize = appearance.value.fontSize;
  terminal.options.fontWeight = appearance.value.fontWeight;
  terminal.options.fontWeightBold = appearance.value.fontWeightBold;
  terminal.options.lineHeight = appearance.value.lineHeight;
  terminal.options.cursorStyle = appearance.value.cursorStyle;
  terminal.options.cursorBlink = appearance.value.cursorBlink;
  terminal.options.minimumContrastRatio = appearance.value.minimumContrastRatio;
  terminal.options.scrollback = behavior.value.scrollback;
  terminal.options.scrollSensitivity = behavior.value.scrollSensitivity;
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

    registerTerminal(sessionId, {
      terminal,
      clear: () => terminal.clear(),
      openSearch,
    });

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
    :style="{ backgroundColor: colorScheme.background }"
  >
    <div ref="container" class="absolute top-4 left-4 right-4 bottom-4" />
    <div
      v-if="searchOpen"
      class="absolute top-6 right-6 z-20 flex items-center gap-1 rounded-lg border bg-popover p-1 shadow-md"
    >
      <Input
        ref="searchInput"
        v-model="searchTerm"
        placeholder="Search..."
        class="h-7 w-48 text-sm"
        @keydown.enter.exact="findNext()"
        @keydown.enter.shift="findPrev()"
        @keydown.escape="closeSearch()"
      />
      <Button variant="ghost" size="icon-xs" :disabled="!hasMatches" @click="findPrev()">
        <ChevronUpIcon />
      </Button>
      <Button variant="ghost" size="icon-xs" :disabled="!hasMatches" @click="findNext()">
        <ChevronDownIcon />
      </Button>
      <Button variant="ghost" size="icon-xs" @click="closeSearch()">
        <XIcon />
      </Button>
    </div>
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
