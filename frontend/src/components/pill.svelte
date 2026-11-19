<script lang="ts">
	import type { Snippet } from 'svelte';

	/** A static tag (rebrush .tag): round, light fill, not interactive. */
	let { text = '', children }: { text?: string; children?: Snippet } = $props();
</script>

<span class="tag">
	{#if text}{text}{:else}{@render children?.()}{/if}
</span>
