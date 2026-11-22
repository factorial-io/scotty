<script lang="ts">
	import { resolve } from '$app/paths';
	import TimeAgo from './time-ago.svelte';
	import TaskStatusPill from './task-status-pill.svelte';
	import { formatDuration } from '../lib/duration';
	import { operationTitle } from '../lib/operationTitle';
	import type { TaskDetail } from '../types';

	let { taskList = [], withApp = true }: { taskList?: TaskDetail[]; withApp?: boolean } =
		$props();

	let sorted = $derived(
		[...taskList].sort(
			(a, b) => new Date(b.start_time).getTime() - new Date(a.start_time).getTime()
		)
	);
</script>

<div class="tw">
	<table>
		<thead>
			<tr>
				<th>Task</th>
				{#if withApp}<th>App</th>{/if}
				<th>Operation</th>
				<th>Status</th>
				<th class="num">Started</th>
				<th class="num">Duration</th>
			</tr>
		</thead>
		<tbody>
			{#each sorted as task (task.id)}
				{@const title = operationTitle(task.operation, task.app_name)}
				<tr>
					<td
						><a
							class="link name mono"
							href={resolve('/tasks/[slug]', { slug: task.id })}>{task.id}</a
						></td
					>
					{#if withApp}
						<td>
							{#if task.app_name}
								<a
									class="link"
									href={resolve('/dashboard/[slug]', { slug: task.app_name })}
									>{task.app_name}</a
								>
							{:else}<span class="meta">—</span>{/if}
						</td>
					{/if}
					{#if title}<td>{title}</td>{:else}<td class="mono">{task.command}</td>{/if}
					<td><TaskStatusPill status={task.state} /></td>
					<td class="num mono"><TimeAgo dateString={task.start_time} /></td>
					<td class="num mono"
						>{task.finish_time
							? formatDuration(task.start_time, task.finish_time)
							: '—'}</td
					>
				</tr>
			{/each}
		</tbody>
	</table>
</div>
