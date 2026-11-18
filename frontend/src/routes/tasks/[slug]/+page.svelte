<script lang="ts">
	import { resolve } from '$app/paths';
	import UnifiedOutput from '../../../components/unified-output.svelte';
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
	import { apps } from '../../../stores/appsStore';

	export let data: TaskDetail;

	let taskOutputRequested = false;

	// Get reactive task output store for this task
	$: taskOutput = getTaskOutput(data.id);

	// Request task output when WebSocket is authenticated
	$: {
		if ($isAuthenticated && !taskOutputRequested) {
			webSocketStore.requestTaskOutputStream(data.id, true);
			taskOutputRequested = true;
		}
	}

	onMount(() => {
		setTitle(`Task ${data.id}`);

		// Initialize task output state
		initializeTaskOutput(data.id);
	});

	onDestroy(() => {
		// Clean up WebSocket subscription
		webSocketStore.stopTaskOutputStream(data.id);
		setTaskOutputSubscribed(data.id, false);
	});

	// Update task details when tasks store changes
	onDestroy(
		tasks.subscribe((new_tasks) => {
			data = new_tasks[data.id] ?? data;
		})
	);

	$: appExists =
		!data.app_name || $apps.length === 0 || $apps.some((a) => a.name === data.app_name);
	const fmt = (d: string) => new Date(d).toLocaleString();
</script>

<div class="ph">
	<div class="ph-row">
		<h1>
			<span class="mono" style="font:inherit">{data.command}</span>{#if data.app_name}
				on {data.app_name}{/if}
		</h1>
	</div>
	<div class="chips">
		<TaskStatusPill status={data.state} />
		{#if $taskOutput.subscribed && data.state === 'Running'}
			<span class="st info"><i class="sw"></i><span class="w">Live</span></span>
		{/if}
	</div>
</div>

<dl class="dl">
	<div>
		<dt>Task</dt>
		<dd class="mono">{data.id}</dd>
	</div>
	<div>
		<dt>App</dt>
		<dd>
			{#if !data.app_name}
				<span class="meta">—</span>
			{:else if appExists}
				<a class="link" href={resolve('/dashboard/[slug]', { slug: data.app_name })}
					>{data.app_name}</a
				>
			{:else}
				{data.app_name} <span class="meta">(destroyed)</span>
			{/if}
		</dd>
	</div>
	<div>
		<dt>Started</dt>
		<dd class="mono">{fmt(data.start_time)} · <TimeAgo dateString={data.start_time} /></dd>
	</div>
	<div>
		<dt>Finished</dt>
		<dd class="mono">{data.finish_time ? fmt(data.finish_time) : '—'}</dd>
	</div>
</dl>

<section class="sec">
	<div class="sec-h"><h2>Output</h2></div>
	<UnifiedOutput
		lines={$taskOutput.lines}
		heading=""
		loading={$taskOutput.loading}
		maxHeight="600px"
	/>
</section>
