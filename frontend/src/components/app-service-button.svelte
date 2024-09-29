<script>
	import ArrowUpRight from '@iconify-icons/heroicons/arrow-up-right';
	import Icon from '@iconify/svelte';
	export let status;
	export let service;
	export let property = 'service';
</script>

{#if status !== 'Running'}
	<button class="btn btn-xs" disabled>{service.service}</button>
{:else}
	<a class="btn btn-xs" href={service.url} target="_"
		><Icon icon={ArrowUpRight} />{service[property]}</a
	>
{/if}
