<script lang="ts">
	import { onMount } from "svelte";
	import { browser } from "$app/environment";
	import { page } from "$app/stores";
	import { Button } from "$lib/components/ui/button";
	import Editor from "$lib/components/Editor.svelte";
	import Output from "$lib/components/Output.svelte";
	import { initWasm, getWasm, type RunResult } from "$lib/wasm";

	const DEFAULT_CODE = `x := 3
y := 7
z := (x + y) / 2
z`;

	let code = $state(DEFAULT_CODE);
	let result: RunResult | null = $state(null);
	let loading = $state(false);
	let wasmReady = $state(false);

	onMount(async () => {
		// Load code from URL if present
		const urlCode = $page.url.searchParams.get("code");
		if (urlCode) {
			try {
				code = atob(urlCode);
			} catch {
				// Invalid base64, use default
			}
		}

		// Initialize WASM
		await initWasm();
		wasmReady = true;
	});

	function runCode() {
		const wasm = getWasm();
		if (!wasm) return;

		loading = true;
		// Use setTimeout to allow UI to update
		setTimeout(() => {
			result = wasm.run(code);
			loading = false;
		}, 0);
	}

	function share() {
		const encoded = btoa(code);
		const url = new URL(window.location.href);
		url.searchParams.set("code", encoded);
		// Update browser URL
		history.replaceState({}, "", url.toString());
		navigator.clipboard.writeText(url.toString());
	}

	function handleKeydown(e: KeyboardEvent) {
		// Cmd/Ctrl+Enter to run
		if ((e.metaKey || e.ctrlKey) && e.key === "Enter") {
			e.preventDefault();
			runCode();
		}
	}
</script>

<svelte:window onkeydown={handleKeydown} />

<div class="h-screen flex flex-col bg-background text-foreground">
	<header
		class="flex items-center justify-between px-4 py-3 border-b border-border"
	>
		<h1 class="text-xl font-title text-primary">toy language playground</h1>
		<div class="flex gap-2">
			<Button variant="secondary" size="sm" onclick={share}>Share</Button>
			<Button size="sm" onclick={runCode} disabled={!wasmReady}>
				{wasmReady ? "Run" : "Loading..."}
			</Button>
		</div>
	</header>

	<main class="flex-1 grid grid-cols-2 gap-4 p-4 min-h-0">
		<div class="min-h-0">
			{#if browser}
				<Editor bind:value={code} />
			{/if}
		</div>
		<div class="min-h-0">
			<Output {result} {loading} />
		</div>
	</main>

	<footer
		class="px-4 py-2 text-xs text-muted-foreground border-t border-border"
	>
		<span>Cmd/Ctrl+Enter to run</span>
	</footer>
</div>
