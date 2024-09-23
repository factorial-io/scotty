<script>
	import { onMount } from 'svelte';
	import PageHeader from '../../components/page-header.svelte';
	import { requestAllTasks } from '../../stores/tasksStore';
	import { tasks } from '../../stores/tasksStore';
	import TimeAgo from '../../components/time-ago.svelte';
	import TaskStatusPill from '../../components/task-status-pill.svelte';
	onMount(async () => {
		function checkTasks() {
			const hasRunningTask = Object.values($tasks).some((task) => task.state === 'Running');
			const interval = hasRunningTask ? 1000 : 5000;
			setTimeout(async () => {
				await requestAllTasks();
				checkTasks();
			}, interval);
		}
		checkTasks();
		await requestAllTasks();
	});

	let taskList = [];

	$: taskList = Object.values($tasks);
</script>

<PageHeader>
	<h2 class="card-title" slot="header">List of recent tasks</h2>
</PageHeader>
<table class="table">
	<!-- head -->
	<thead>
		<tr>
			<th>Id</th>
			<th>Status</th>
			<th>Started</th>
			<th>Finished</th>
		</tr>
	</thead>
	<tbody>
		{#each taskList.sort((a, b) => new Date(b.start_time) - new Date(a.start_time)) as task}
			<tr>
				<td><a href="/tasks/{task.id}">{task.id}</a></td>
				<td><TaskStatusPill status={task.state} /></td>
				<td><TimeAgo dateString={task.start_time} /></td>
				<td><TimeAgo dateString={task.finish_time} /></td>
			</tr>
		{/each}
	</tbody>
</table>
