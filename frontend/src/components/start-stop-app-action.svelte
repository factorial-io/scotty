<script lang="ts">
	import Icon from '@iconify/svelte';
	import play from '@iconify-icons/heroicons/play-solid';
	import stop from '@iconify-icons/heroicons/stop-solid';
	import { runApp, stopApp, updateAppInfo } from '../stores/appsStore';
	import { monitorTask } from '../stores/tasksStore';

	export let name = '';
	export let status = 'Stopped';

	let task_id: string = '';

	function currentIcon() {
		if (status === 'Running') {
			return stop;
		} else {
			return play;
		}
	}

	async function handleClick() {
		if (task_id !== '') return;
		console.log(name, status);
		if (status === 'Running') {
			task_id = await stopApp(name);
		} else {
			task_id = await runApp(name);
		}
		console.log(task_id);
		monitorTask(task_id, (result) => {
			task_id = '';
			console.log(result);
			updateAppInfo(name);
		});
	}
</script>

<button class="btn btn-circle btn-xs" on:click={handleClick}>
	{#if task_id !== ''}
		<span class="loading loading-spinner"></span>
	{:else}
		<Icon icon={currentIcon()} />
	{/if}
</button>
