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

<!-- URLs are always shown when domains exist (they come from app settings, not
     live containers); the landing page turns a stopped app's URL into a start
     affordance. Non-running services get a muted style instead of being hidden. -->
{#if service.domains.length === 0}
	<button class="btn btn-xs" disabled>{service.service}</button>
{:else}
	{@const running = service.status === 'Running'}
	<!-- eslint-disable svelte/no-navigation-without-resolve -->
	{#each service.domains as domain (domain)}
		<a
			title={running ? domain : `${domain} (service not running)`}
			class="btn btn-xs mr-2 mb-2 {running ? '' : 'btn-ghost opacity-60'}"
			href={url(domain, service.use_tls)}
			target="_blank"
			rel="external"><Icon icon={ArrowUpRight} />{title(domain)}</a
		>
	{/each}
	<!-- eslint-enable svelte/no-navigation-without-resolve -->
{/if}
