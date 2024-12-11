<script lang="ts">
	export let environment: Map<string, string> | null;
</script>

{#if environment === null || (environment && Object.keys(environment).length === 0)}
	None
{:else}
	<dl class="flex flex-col gap-1">
		{#each Object.entries(environment) as [key, value]}
			<div class="flex">
				<dt class="font-mono">{key}=</dt>
				<dd class="font-mono break-all">{value}</dd>
			</div>
		{/each}
	</dl>
{/if}
