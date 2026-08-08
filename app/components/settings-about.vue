<script setup lang="ts">
import { DownloadIcon, RefreshCwIcon } from '@lucide/vue';
import pkg from '../../package.json';

const appVersion = pkg.version;
const { checking, installing, progress, pendingUpdate, checkForUpdates, installUpdate } =
  useUpdater();
</script>

<template>
  <div class="flex flex-col gap-5">
    <div class="flex items-center gap-3">
      <span class="text-muted-foreground text-sm">v{{ appVersion }}</span>
      <Button
        v-if="pendingUpdate"
        size="sm"
        variant="outline"
        class="h-7"
        :disabled="installing"
        @click="installUpdate"
      >
        <DownloadIcon class="size-3.5" />
        {{ installing ? `Installing… ${progress}%` : `Install v${pendingUpdate.version}` }}
      </Button>
      <Button
        v-else
        size="sm"
        variant="outline"
        class="h-7"
        :disabled="checking"
        @click="checkForUpdates(true)"
      >
        <RefreshCwIcon class="size-3.5" :class="{ 'animate-spin': checking }" />
        {{ checking ? 'Checking…' : 'Check for updates' }}
      </Button>
    </div>

    <p class="text-muted-foreground text-sm leading-relaxed">
      A fast, modern SSH client and terminal emulator built with Tauri, xterm.js, and Rust. Manage
      connections, keys, and sessions across all your devices. Free and open source, forever.
    </p>

    <div class="flex flex-wrap gap-2">
      <Button size="sm" variant="link" class="p-0" as-child>
        <a href="https://github.com/Hrdtr/sheil" target="_blank" rel="noopener noreferrer">
          Star on GitHub
        </a>
      </Button>
      <Button size="sm" variant="link" class="p-0" as-child>
        <a href="https://github.com/sponsors/Hrdtr" target="_blank" rel="noopener noreferrer">
          Support
        </a>
      </Button>
    </div>

    <Separator />

    <p class="text-muted-foreground text-sm">
      Built by
      <a
        href="https://hrdtr.dev"
        target="_blank"
        rel="noopener noreferrer"
        class="hover:text-foreground transition-colors"
      >
        Herdi Tr.
      </a>
      &middot; GPLv3
    </p>
  </div>
</template>
