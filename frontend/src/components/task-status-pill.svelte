<script>
	import Pill from './pill.svelte';

	export let status = 'Running';

	$: statusColor =
		status === 'Failed'
			? 'bg-red-500'
			: status === 'Finished'
				? 'bg-green-500'
				: status === 'Running'
					? 'bg-secondary'
					: 'bg-gray-300';
</script>

<Pill text={status} color={statusColor} />
