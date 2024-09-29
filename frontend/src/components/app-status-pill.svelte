<script>
	export let status = 'Running';

	$: statusColor = status.toLowerCase() === 'running' ? 'bg-green-500' : 'bg-gray-300';
</script>

<span class="text-white px-3 py-1 rounded-full text-xs {statusColor}">{status}</span>
