<script lang="ts">
	import TimeAgo from './time-ago.svelte';
	import TaskStatusPill from './task-status-pill.svelte';

	import type { TaskDetail } from '../types';

	export let taskList: TaskDetail[] = [];
</script>

<table class="table">
	<!-- head -->
	<thead>
		<tr>
			<th>Id</th>
			<th>Status</th>
			<th>Started</th>
			<th>Finished</th>
			<th>App</th></tr
		>
	</thead>
	<tbody>
		{#each taskList.sort((a, b) => new Date(b.start_time).getTime() - new Date(a.start_time).getTime()) as task}
			<tr>
				<td><a class="link-primary" href="/tasks/{task.id}">{task.id}</a></td>
				<td><TaskStatusPill status={task.state} /></td>
				<td><TimeAgo dateString={task.start_time} /></td>
				<td><TimeAgo dateString={task.finish_time} /></td>
				<td><a class="link-secondary" href="/dashboard/{task.app_name}">{task.app_name}</a></td>
			</tr>
		{/each}
	</tbody>
</table>
