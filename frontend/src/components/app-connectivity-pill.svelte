<script lang="ts">
	import StatusMark, { type Tone } from './status-mark.svelte';
	import type { LoadBalancerConnectivity } from '../types';

	let { connectivity = 'Unknown' }: { connectivity?: LoadBalancerConnectivity } = $props();

	// Only the states the server actually observed are worth showing. An
	// always-present neutral mark would train people to ignore it, and the point
	// of this indicator is that an unreachable app is noticeable.
	const states: Partial<
		Record<LoadBalancerConnectivity, { label: string; tone: Tone; title: string }>
	> = {
		Connected: {
			label: 'Routable',
			tone: 'ok',
			title: 'The load balancer is attached to this app’s proxy network'
		},
		Disconnected: {
			label: 'Not routable',
			tone: 'err',
			title: 'The load balancer is not attached to this app’s proxy network, so requests to its domains do not reach it'
		},
		LoadBalancerUnavailable: {
			label: 'LB unavailable',
			tone: 'warn',
			title: 'The load balancer container could not be reached, so no app is routable'
		}
	};

	let state = $derived(states[connectivity]);
</script>

{#if state}
	<StatusMark tone={state.tone} label={state.label} title={state.title} />
{/if}
