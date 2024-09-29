<script>
	import { onMount } from 'svelte';
	import PageHeader from '../../components/page-header.svelte';
	import { requestAllTasks } from '../../stores/tasksStore';
	import { tasks } from '../../stores/tasksStore';
	import TasksTable from '../../components/tasks-table.svelte';
	onMount(async () => {
		await requestAllTasks();
	});

	let taskList = [];

	$: taskList = Object.values($tasks);
</script>

<PageHeader>
	<h2 class="card-title" slot="header">List of recent tasks</h2>
</PageHeader>
<TasksTable {taskList} />
