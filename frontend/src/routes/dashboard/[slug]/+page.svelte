<script lang="ts">
	import PageHeader from '../../../components/page-header.svelte';
	import AppStatusPill from '../../../components/app-status-pill.svelte';
	import TimeAgo from '../../../components/time-ago.svelte';
	import { dispatchAppCommand, updateAppInfo } from '../../../stores/appsStore';
	import { monitorTask } from '../../../stores/tasksStore';
	import type { App, TaskDetail } from '../../../types';
	import TasksTable from '../../../components/tasks-table.svelte';
	import { tasks } from '../../../stores/tasksStore';
	import AppServiceButton from '../../../components/app-service-button.svelte';

	/** @type {import('./$types').PageData} */
	export let data: App;

	let actions = ['Run', 'Stop', 'Purge', 'Rebuild'];
	if (data.settings) {
		actions.push('Destroy');
	}
	let current_task: string | null = null;
	let current_action: string | null = null;
	async function handleClick(action: string) {
		current_action = action;
		current_task = await dispatchAppCommand(action.toLowerCase(), data.name);
		monitorTask(current_task, async () => {
			current_task = null;
			current_action = null;
			data = (await updateAppInfo(data.name)) as App;
		});
	}

	let app_tasks: TaskDetail[] = [];
	tasks.subscribe((new_tasks) => {
		app_tasks = Object.values(new_tasks).filter((t) => t.app_name === data.name);
	});
</script>

<PageHeader>
	<h2 class="card-title" slot="header">App-Details for {data.name}</h2>
	<div slot="meta">
		<AppStatusPill status={data.status} />
	</div>
</PageHeader>

<h3 class="text-xl mt-16 mb-4">Available Actions</h3>
<div class="join">
	{#each actions as action}
		<button
			disabled={current_task !== null}
			class="btn btn-sm join-item"
			on:click={() => handleClick(action)}
			>{#if action === current_action}
				<span class="loading loading-spinner"></span>
			{/if}{action}</button
		>
	{/each}
</div>
<h3 class="text-xl mt-16 mb-4">Available Services</h3>
<table class="table">
	<thead>
		<th>Name</th>
		<th>Status</th>
		<th>Url</th>
		<th>Started</th>
	</thead>
	<tbody>
		{#each data.services as service}
			<tr>
				<td>{service.service}</td>
				<td><AppStatusPill status={service.status || 'unknown'} /></td>
				<td
					>{#if service.url}
						<AppServiceButton property="domain" {service} status={data.status} />
					{:else}
						--
					{/if}
				</td>
				<td>
					<TimeAgo dateString={service.started_at} />
				</td>
			</tr>{/each}
	</tbody>
</table>
<h3 class="text-xl mt-16 mb-4">Latest task-invocations</h3>
<TasksTable taskList={app_tasks} />
