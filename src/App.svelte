<script lang="ts">
	import TerminalView from '$lib/components/terminal-view.svelte';
</script>

<main class="flex min-h-screen items-center justify-center bg-gray-950 p-4">
	<div class="h-[600px] w-full max-w-3xl">
		<TerminalView sessionId={null} />
	</div>
</main>
