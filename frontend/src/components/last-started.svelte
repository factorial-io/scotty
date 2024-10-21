<script lang="ts">
	import type { AppService } from '../types';
	import TimeAgo from './time-ago.svelte';

	export let services: AppService[] = [];

	let lastStarted: string | null = null;
	$: {
		if (services.length > 0) {
			lastStarted =
				services.reduce(
					(oldest, service) => {
						if (!oldest || new Date(service.started_at) < new Date(oldest.started_at)) {
							return service;
						}
						return oldest;
					},
					null as AppService | null
				)?.started_at ?? null;
		}
	}
</script>

<TimeAgo dateString={lastStarted}></TimeAgo>
