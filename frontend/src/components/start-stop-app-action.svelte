<script lang="ts">
	import Icon from '@iconify/svelte';
	import play from '@iconify-icons/heroicons/play-solid';
	import stop from '@iconify-icons/heroicons/stop-solid';
	import { runApp, stopApp, updateAppInfo } from '../stores/appsStore';
	import { monitorTask } from '../stores/tasksStore';

	export let name = '';
	export let status = 'Stopped';

	let task_id: string = '';

	$: currentIcon = status === 'Running' ? stop : play;

	async function handleClick() {
		if (task_id !== '') return;
		task_id = await (status === 'Running' ? stopApp(name) : runApp(name));
		monitorTask(task_id, () => {
			task_id = '';
			updateAppInfo(name);
		});
	}
</script>

<button class="btn btn-circle btn-xs" on:click={handleClick}>
	{#if task_id !== ''}
		<span class="loading loading-spinner"></span>
	{:else}
		<Icon icon={currentIcon} />
	{/if}
</button>
