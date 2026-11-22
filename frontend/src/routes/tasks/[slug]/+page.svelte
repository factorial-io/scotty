<script lang="ts">
	import { resolve } from '$app/paths';
	import LogViewer from '../../../components/log-viewer.svelte';
	import TaskProgress from '../../../components/task-progress.svelte';
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
	import { operationTitle } from '../../../lib/operationTitle';

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

	$: title = operationTitle(data.operation, data.app_name);
	$: setTitle(title ?? `Task ${data.id}`);

	onMount(() => {
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
			{#if title}
				{title}
			{:else}
				<span class="mono" style="font:inherit">{data.command}</span>{data.app_name
					? ` on ${data.app_name}`
					: ''}
			{/if}
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
	<div>
		<dt>Last command</dt>
		<dd class="mono last-command">{data.command || '—'}</dd>
	</div>
</dl>

<div class="full-progress"><TaskProgress taskId={data.id} /></div>

<section class="sec">
	<div class="sec-h"><h2>Output</h2></div>
	<LogViewer
		lines={$taskOutput.lines}
		live={data.state === 'Running' && $taskOutput.subscribed}
	/>
</section>

<style>
	.last-command {
		white-space: pre-wrap;
		overflow-wrap: anywhere;
	}
	.full-progress {
		margin-top: 16px;
	}
	.full-progress :global(.progress) {
		max-width: none;
		height: 6px;
	}
</style>
