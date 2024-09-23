<script>
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

<span class="text-white px-3 py-1 rounded-full text-xs {statusColor}">{status}</span>
