<script lang="ts">
	import ArrowUpRight from '@iconify-icons/ph/arrow-up-right';
	import Icon from '@iconify/svelte';
	import type { AppService } from '../types';
	export let status;
	export let service: AppService;
	export let property = 'service';

	function url(domain: string, use_tls: boolean) {
		return `${use_tls ? 'https' : 'http'}://${domain}`;
	}
	function title(domain: string) {
		return property === 'domain' ? domain : service[property as keyof AppService];
	}
</script>

{#if status !== 'Running'}
	<button class="btn btn-xs" disabled>{service.service}</button>
{:else}
	{#each service.domains as domain (domain)}
		<a
			title={domain}
			class="btn btn-xs mr-2 mb-2"
			href={url(domain, service.use_tls)}
			target="_blank"><Icon icon={ArrowUpRight} />{title(domain)}</a
		>
	{/each}
{/if}
