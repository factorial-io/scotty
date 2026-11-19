<script lang="ts">
	import type { AppService } from '../types';
	import { lastStartedAt } from '../lib/appList';
	import TimeAgo from './time-ago.svelte';

	let { services = [] }: { services?: AppService[] } = $props();
</script>

<TimeAgo dateString={lastStartedAt(services)} />
