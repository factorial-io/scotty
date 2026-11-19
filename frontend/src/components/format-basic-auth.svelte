<script lang="ts">
	import Icon from '@iconify/svelte';

	let { auth }: { auth: string[] | null } = $props();
	let revealed = $state(false);
</script>

{#if auth === null}
	None
{:else}
	<!-- A fixed mask, so it does not give away the password's length (design D8). -->
	<span class="mono">{auth[0]} / {revealed ? auth[1] : '••••••'}</span>
	<button
		class="ibtn"
		aria-pressed={revealed}
		aria-label={revealed ? 'Hide password' : 'Show password'}
		title={revealed ? 'Hide password' : 'Show password'}
		onclick={() => (revealed = !revealed)}
	>
		<Icon icon={revealed ? 'ph:eye-slash' : 'ph:eye'} class="ic sm" aria-hidden="true" />
	</button>
{/if}
