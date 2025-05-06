<script>
	import Pill from './pill.svelte';

	export let status = 'Running';

	$: statusColor =
		status.toLowerCase() === 'running' ? 'bg-green-500' : 'bg-gray-300 dark:bg-gray-700';
</script>

<Pill text={status} color={statusColor} />
