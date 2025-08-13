<script lang="ts">
	import Icon from '@iconify/svelte';

	export let name: string;
	export let key: string;
	export let sortBy: string;
	export let onsetSort: (event: { sortBy: string }) => void;

	function setSort(sortValue: string) {
		onsetSort({ sortBy: sortValue });
	}
</script>

<button class="link" on:click={() => setSort(key)}
	>{name}
	{#if sortBy == key}<Icon class="float-right" icon="ph:arrow-up" />
	{/if}</button
>
