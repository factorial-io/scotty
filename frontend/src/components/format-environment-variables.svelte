<script lang="ts">
	let { environment }: { environment: Map<string, string> | Record<string, string> | null } =
		$props();
	let entries = $derived(
		Object.entries(environment ?? {}).sort(([a], [b]) => a.localeCompare(b))
	);
</script>

{#if entries.length === 0}
	<span class="meta">None</span>
{:else}
	<div class="env mono">
		{#each entries as [key, value] (key)}
			<span>{key}</span><span>{value}</span>
		{/each}
	</div>
{/if}
