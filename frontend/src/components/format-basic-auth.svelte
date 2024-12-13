<script lang="ts">
	export let auth: string[] | null;
</script>

{#if auth === null}
	None
{:else}
	<div class="font-mono">
		<span>{auth[0]}</span> /
		<span>{auth[1]}</span>
	</div>
{/if}
