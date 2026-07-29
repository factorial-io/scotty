<script lang="ts">
	import Pill from './pill.svelte';
	import type { LoadBalancerConnectivity } from '../types';

	export let connectivity: LoadBalancerConnectivity = 'Unknown';

	// Only the states the server actually observed are worth showing. An
	// always-present neutral pill would train people to ignore it, and the point
	// of this indicator is that an unreachable app is noticeable.
	$: label =
		connectivity === 'Connected'
			? 'Routable'
			: connectivity === 'Disconnected'
				? 'Not routable'
				: connectivity === 'LoadBalancerUnavailable'
					? 'LB unavailable'
					: null;

	$: color =
		connectivity === 'Connected'
			? 'bg-green-500'
			: connectivity === 'Disconnected'
				? 'bg-red-500'
				: 'bg-amber-500';

	$: title =
		connectivity === 'Connected'
			? 'The load balancer is attached to this app’s proxy network'
			: connectivity === 'Disconnected'
				? 'The load balancer is not attached to this app’s proxy network, so requests to its domains do not reach it'
				: 'The load balancer container could not be reached, so no app is routable';
</script>

{#if label}
	<span {title}>
		<Pill text={label} {color} />
	</span>
{/if}
