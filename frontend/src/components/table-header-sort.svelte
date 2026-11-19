<script lang="ts">
	import Icon from '@iconify/svelte';

	interface Props {
		name: string;
		key: string;
		sortBy: string;
		onsetSort?: (event: { sortBy: string }) => void;
	}

	let { name, key, sortBy, onsetSort = () => {} }: Props = $props();
</script>

<!-- Styled by `th button` in scotty.css. Ascending only. -->
<button aria-pressed={sortBy === key} onclick={() => onsetSort({ sortBy: key })}>
	{name}
	{#if sortBy === key}<Icon icon="ph:caret-down" class="ic sm" aria-hidden="true" />{/if}
</button>
