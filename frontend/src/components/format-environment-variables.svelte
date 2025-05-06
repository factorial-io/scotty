<script lang="ts">
	export let environment: Map<string, string> | null;
</script>

{#if environment === null || (environment && Object.keys(environment).length === 0)}
	None
{:else}
	<dl class="flex flex-col gap-1">
		{#each Object.entries(environment).sort( ([a], [b]) => a.localeCompare(b) ) as [key, value] (key)}
			<div class="flex">
				<dt class="font-mono">{key}&nbsp;=&nbsp;</dt>
				<dd class="font-mono wrap-anywhere">{value}</dd>
			</div>
		{/each}
	</dl>
{/if}

<style>
	.wrap-anywhere {
		word-break: break-all;
		overflow-wrap: anywhere;
		white-space: pre-wrap;
		max-width: 100%;
	}
</style>
