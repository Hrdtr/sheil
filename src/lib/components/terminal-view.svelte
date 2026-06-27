<script lang="ts">
	import { Terminal } from 'xterm';
	import { WebglAddon } from '@xterm/addon-webgl';
	import { FitAddon } from '@xterm/addon-fit';
	import { WebLinksAddon } from '@xterm/addon-web-links';
	import { listen, type UnlistenFn } from '@tauri-apps/api/event';
	import { Card, Content, Header, Title } from '$lib/components/ui/card/index';
	import {
		sshOpenChannel,
		sshWrite,
		sshResize,
		sshCloseChannel,
	} from '$lib/commands.svelte';

	let { sessionId }: { sessionId: string | null } = $props();

	let container: HTMLDivElement | undefined = undefined;

	const terminal = new Terminal({
		cursorBlink: true,
		theme: {
			background: '#1e1e2e',
			foreground: '#cdd6f4',
			cursor: '#f5e0dc',
			selectionBackground: '#585b70',
			black: '#45475a',
			red: '#f38ba8',
			green: '#a6e3a1',
			yellow: '#f9e2af',
			blue: '#89b4fa',
			magenta: '#f5c2e7',
			cyan: '#94e2d5',
			white: '#bac2de',
			brightBlack: '#585b70',
			brightRed: '#f38ba8',
			brightGreen: '#a6e3a1',
			brightYellow: '#f9e2af',
			brightBlue: '#89b4fa',
			brightMagenta: '#f5c2e7',
			brightCyan: '#94e2d5',
			brightWhite: '#a6adc8'
		},
		fontFamily: '"JetBrains Mono", "Fira Code", "Cascadia Code", monospace',
		fontSize: 14,
		lineHeight: 1.2
	});

	const fitAddon = new FitAddon();
	const webLinksAddon = new WebLinksAddon();
	terminal.loadAddon(fitAddon);
	terminal.loadAddon(webLinksAddon);

	try {
		terminal.loadAddon(new WebglAddon());
	} catch {
		// WebGL not available — fall back to default canvas renderer
	}

	terminal.writeln('Welcome to Sheil Terminal');
	terminal.writeln('');

	let resizeObserver: ResizeObserver | undefined;

	$effect(() => {
		if (!container) return;

		terminal.open(container);
		fitAddon.fit();

		resizeObserver = new ResizeObserver(() => {
			fitAddon.fit();
		});
		resizeObserver.observe(container);

		return () => {
			resizeObserver?.disconnect();
			terminal.dispose();
		};
	});

	$effect(() => {
		if (!sessionId) return;

		const cols = terminal.cols;
		const rows = terminal.rows;

		sshOpenChannel(sessionId, cols, rows).catch((e) => {
			terminal.writeln(`\r\n\x1b[31mPTY error: ${e}\x1b[0m`);
		});

		const dataDispose = terminal.onData((data) => {
			const encoder = new TextEncoder();
			sshWrite(sessionId, encoder.encode(data)).catch(() => {});
		});

		const resizeDispose = terminal.onResize(({ cols, rows }) => {
			sshResize(sessionId, cols, rows).catch(() => {});
		});

		let unlistenOutput: UnlistenFn | undefined;
		let unlistenExit: UnlistenFn | undefined;

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

		return () => {
			dataDispose.dispose();
			resizeDispose.dispose();
			unlistenOutput?.();
			unlistenExit?.();
			sshCloseChannel(sessionId).catch(() => {});
		};
	});
</script>

<Card class="h-full w-full">
	<Header class="pt-4 pb-2">
		<Title class="text-sm font-medium">Terminal</Title>
	</Header>
	<Content class="h-full p-0">
		<div
			bind:this={container}
			class="h-full min-h-[400px] w-full overflow-hidden rounded-b-lg"
		></div>
	</Content>
</Card>

<!-- svelte-ignore css_unused_selector -->
<style global>
	@import 'xterm/css/xterm.css';
</style>
