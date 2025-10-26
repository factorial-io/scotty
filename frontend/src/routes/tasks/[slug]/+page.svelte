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
	import { webSocketStore } from '../../../stores/webSocketStore';
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
	<h2 class="card-title" slot="header">Task-Details</h2>
	<div slot="meta">
		<div class="flex items-center gap-2">
			<TaskStatusPill status={data.state} />
			{#if $taskOutput.subscribed}
				<span class="badge badge-info badge-sm">Live</span>
			{/if}
		</div>
	</div>
</PageHeader>

<!-- Breadcrumb Navigation -->
<div class="text-sm breadcrumbs mb-4">
	<ul>
		<li><a href={resolve('/dashboard')}>Dashboard</a></li>
		<li><a href={resolve(`/dashboard/${data.app_name}`)}>{data.app_name}</a></li>
		<li>{data.id}</li>
	</ul>
</div>

<!-- Task Information Section -->
<h3 class="text-xl mt-8 mb-4">Task Information</h3>
<table class="table table-zebra">
	<tbody>
		<tr>
			<td class="text-gray-500 w-48"><strong>Task ID</strong></td>
			<td>{data.id}</td>
		</tr>
		<tr>
			<td class="text-gray-500"><strong>Status</strong></td>
			<td><TaskStatusPill status={data.state} /></td>
		</tr>
		<tr>
			<td class="text-gray-500"><strong>Started</strong></td>
			<td><TimeAgo dateString={data.start_time} /></td>
		</tr>
		<tr>
			<td class="text-gray-500"><strong>Finished</strong></td>
			<td><TimeAgo dateString={data.finish_time} /></td>
		</tr>
		<tr>
			<td class="text-gray-500"><strong>App</strong></td>
			<td>
				<a class="link link-primary" href={resolve(`/dashboard/${data.app_name}`)}
					>{data.app_name}</a
				>
			</td>
		</tr>
	</tbody>
</table>

<!-- Task Output Section -->
<h3 class="text-xl mt-8 mb-4">Task Output</h3>
<UnifiedOutput
	lines={$taskOutput.lines}
	heading=""
	loading={$taskOutput.loading}
	maxHeight="600px"
/>
