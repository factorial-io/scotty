<script lang="ts">
	import Icon from '@iconify/svelte';
	import type { AppService } from '../types';

	let {
		service,
		property = 'service'
	}: { service: AppService; property?: 'service' | 'domain' } = $props();

	const url = (domain: string) => `${service.use_tls ? 'https' : 'http'}://${domain}`;
	let running = $derived(service.status === 'Running');
</script>

<!-- A stopped service still links to its domain, in a muted style: the landing
     page there offers to start the app (decision 2). Services without a domain
     render nothing; callers show a dash. -->
<!-- eslint-disable svelte/no-navigation-without-resolve -->
{#each service.domains as domain (domain)}
	<a
		class="btn outline sm"
		class:muted={!running}
		title={running ? domain : `${domain} (not running, opens the start page)`}
		href={url(domain)}
		target="_blank"
		rel="external"
		>{property === 'domain' ? domain : service.service}<Icon
			icon="ph:arrow-square-out"
			class="ic sm"
			aria-hidden="true"
		/></a
	>
{/each}
