<script lang="ts">
	import { resolve } from '$app/paths';
	import UnifiedOutput from '../../../components/unified-output.svelte';
	import PageHeader from '../../../components/page-header.svelte';
	import TaskStatusPill from '../../../components/task-status-pill.svelte';
	import TimeAgo from '../../../components/time-ago.svelte';
	import { tasks } from '../../../stores/tasksStore';
	import {
		getTaskOutput,
		initializeTaskOutput,
		setTaskOutputSubscribed
	} from '../../../stores/taskOutputStore';
	import { webSocketStore, isConnected } from '../../../stores/webSocketStore';
	import { isAuthenticated } from '../../../stores/sessionStore';
	import type { TaskDetail } from '../../../types';
	import { onMount, onDestroy } from 'svelte';
	import { setTitle } from '../../../stores/titleStore';

	export let data: TaskDetail;

	let taskOutputRequested = false;

	// Get reactive task output store for this task
	$: taskOutput = getTaskOutput(data.id);

	// Debug reactive statement to log task output state changes
	$: {
		console.log('Task output state for', data.id, ':', $taskOutput);
	}

	// Request task output when WebSocket is authenticated
	$: {
		if ($isAuthenticated && !taskOutputRequested) {
			console.log('WebSocket authenticated, requesting task output for:', data.id);
			webSocketStore.requestTaskOutputStream(data.id, true);
			taskOutputRequested = true;
		}
	}

	onMount(() => {
		setTitle(`Task: ${data.id} for ${data.app_name}`);

		console.log('Task details page mounted for task:', data.id);
		console.log('Task data:', data);

		// Initialize task output state
		initializeTaskOutput(data.id);
	});

	onDestroy(() => {
		// Clean up WebSocket subscription
		webSocketStore.stopTaskOutputStream(data.id);
		setTaskOutputSubscribed(data.id, false);
	});

	// Update task details when tasks store changes
	tasks.subscribe((new_tasks) => {
		data = Object.values(new_tasks).find((t) => t.id === data.id) || data;
	});
</script>

<PageHeader>
	<h2 class="card-title" slot="header">
		Task-Details for <br />{data.id}<br />
	</h2>
	<div slot="meta">
		<div class="flex items-center gap-2">
			<TaskStatusPill status={data.state} />
			{#if $taskOutput.subscribed}
				<span class="badge badge-info badge-sm">Live</span>
			{/if}
		</div>
		<div class="mt-2 text-xs text-gray-500">
			Started <TimeAgo dateString={data.start_time} /> <br />
			Finished <TimeAgo dateString={data.finish_time} /> <br />
			App:
			<a class="link-primary" href={resolve(`/dashboard/${data.app_name}`)}>{data.app_name}</a>
		</div>
	</div>
</PageHeader>

<!-- Unified Output Display -->
<UnifiedOutput
	lines={$taskOutput.lines}
	heading="Task Output"
	loading={$taskOutput.loading}
	maxHeight="600px"
/>
