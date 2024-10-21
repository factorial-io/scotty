<script lang="ts">
	import Icon from '@iconify/svelte';

	export let name: string;
	export let key: string;
	export let sortBy: string;

	import { createEventDispatcher } from 'svelte';

	const dispatch = createEventDispatcher();
	function setSort(sortValue: string) {
		dispatch('setSort', { sortBy: sortValue });
	}
</script>

<button class="link" on:click={() => setSort(key)}
	>{name}
	{#if sortBy == key}<Icon class="float-right" icon="heroicons:arrow-small-up-solid" />
	{/if}</button
>
