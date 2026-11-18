<script lang="ts">
	import { onMount } from 'svelte';
	import { requestAllTasks, tasks } from '../../stores/tasksStore';
	import TasksTable from '../../components/tasks-table.svelte';
	import { setTitle } from '../../stores/titleStore';

	onMount(() => {
		setTitle('Tasks');
		requestAllTasks();
	});
</script>

<div class="ph">
	<div class="ph-row"><h1>Tasks</h1></div>
	<p class="lead">Every action Scotty ran on an app, newest first. Running tasks update live.</p>
</div>
<section class="sec">
	{#if Object.keys($tasks).length}
		<TasksTable taskList={Object.values($tasks)} />
	{:else}
		<div class="empty">
			<h2>No tasks yet</h2>
			<p>Tasks appear here when you or scottyctl run an action on an app.</p>
		</div>
	{/if}
</section>
