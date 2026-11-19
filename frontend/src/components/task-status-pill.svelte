<script lang="ts">
	import StatusMark from './status-mark.svelte';

	let { status = 'Running' }: { status?: string } = $props();
</script>

{#if status === 'Running'}
	<StatusMark tone="info" label="Running" icon="spinner" />
{:else if status === 'Finished'}
	<StatusMark tone="ok" label="Finished" icon="ph:check-circle" />
{:else if status === 'Failed'}
	<StatusMark tone="err" label="Failed" icon="ph:warning-circle" />
{:else}
	<StatusMark tone="off" label={status} />
{/if}
