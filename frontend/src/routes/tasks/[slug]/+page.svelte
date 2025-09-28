<script lang="ts">
	import UnifiedOutput from '../../../components/unified-output.svelte';
	import PageHeader from '../../../components/page-header.svelte';
	import TaskStatusPill from '../../../components/task-status-pill.svelte';
	import TimeAgo from '../../../components/time-ago.svelte';
	import { tasks } from '../../../stores/tasksStore';
	import { getTaskOutput, initializeTaskOutput, setTaskOutputSubscribed } from '../../../stores/taskOutputStore';
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
			App: <a class="link-primary" href="/dashboard/{data.app_name}">{data.app_name}</a>
		</div>
	</div>
</PageHeader>

<!-- Output Info -->
{#if $taskOutput.lines.length > 0}
	<div class="mb-4 text-sm text-gray-500">
		{$taskOutput.lines.length} lines
		{#if $taskOutput.totalLines > 0 && $taskOutput.totalLines !== $taskOutput.lines.length}
			(of {$taskOutput.totalLines})
		{/if}
	</div>
{/if}

<!-- Unified Output Display -->
<UnifiedOutput
	lines={$taskOutput.lines}
	heading="Task Output"
	loading={$taskOutput.loading}
	maxHeight="600px"
/>

<!-- Control Buttons -->
<div class="mt-4 flex items-center justify-end">
	<button
		class="btn btn-sm btn-outline"
		class:btn-error={$taskOutput.subscribed && $isConnected}
		class:btn-primary={!$taskOutput.subscribed && $isConnected && data.state === 'Running'}
		class:btn-disabled={!($taskOutput.subscribed && $isConnected) && !(!$taskOutput.subscribed && $isConnected && data.state === 'Running')}
		disabled={!($taskOutput.subscribed && $isConnected) && !(!$taskOutput.subscribed && $isConnected && data.state === 'Running')}
		on:click={() => {
			if ($taskOutput.subscribed && $isConnected) {
				webSocketStore.stopTaskOutputStream(data.id);
			} else if (!$taskOutput.subscribed && $isConnected && data.state === 'Running') {
				webSocketStore.requestTaskOutputStream(data.id, false);
			}
		}}
	>
		{#if $taskOutput.subscribed && $isConnected}
			Stop Stream
		{:else if !$taskOutput.subscribed && $isConnected && data.state === 'Running'}
			Start Stream
		{:else}
			{data.state === 'Running' ? 'Not Connected' : 'Task Not Running'}
		{/if}
	</button>
</div>
