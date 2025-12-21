<script lang="ts">
	import type { RunResult } from "$lib/wasm";

	interface Props {
		result: RunResult | null;
		loading?: boolean;
	}

	let { result, loading = false }: Props = $props();
</script>

<div
	class="h-full w-full border border-border rounded-md overflow-auto bg-card p-4 font-mono text-sm"
>
	{#if loading}
		<span class="text-muted-foreground">Running...</span>
	{:else if result === null}
		<span class="text-muted-foreground">Click "Run" to execute</span>
	{:else if result.success}
		<div>
			<span class="text-success">{result.value}</span>
			{#if result.value_type}
				<span class="text-muted-foreground ml-2"
					>: {result.value_type}</span
				>
			{/if}
		</div>
	{:else}
		<div class="text-destructive">{result.error}</div>
	{/if}
</div>
