<script lang="ts">
	import ArrowUpRight from '@iconify-icons/ph/arrow-up-right';
	import Icon from '@iconify/svelte';
	import type { AppService } from '../types';
	export let service: AppService;
	export let property = 'service';

	function url(domain: string, use_tls: boolean) {
		return `${use_tls ? 'https' : 'http'}://${domain}`;
	}
	function title(domain: string) {
		return property === 'domain' ? domain : service[property as keyof AppService];
	}
</script>

<!-- Gate URL visibility on this service's own container status, so a running
     service shows its URL even when the overall app is not fully Running
     (e.g. a sibling init/one-shot container has already Exited). -->
{#if service.status !== 'Running'}
	<button class="btn btn-xs" disabled>{service.service}</button>
{:else}
	<!-- eslint-disable svelte/no-navigation-without-resolve -->
	{#each service.domains as domain (domain)}
		<a
			title={domain}
			class="btn btn-xs mr-2 mb-2"
			href={url(domain, service.use_tls)}
			target="_blank"
			rel="external"><Icon icon={ArrowUpRight} />{title(domain)}</a
		>
	{/each}
	<!-- eslint-enable svelte/no-navigation-without-resolve -->
{/if}
