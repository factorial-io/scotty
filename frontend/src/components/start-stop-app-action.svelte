<script lang="ts">
	import Icon from '@iconify/svelte';
	import play from '@iconify-icons/ph/play-circle-fill';
	import stop from '@iconify-icons/ph/stop-circle-fill';
	import unsupported from '@iconify-icons/heroicons/no-symbol';
	import errorIcon from '@iconify-icons/heroicons/exclamation-triangle';
	import { runApp, stopApp, updateAppInfo } from '../stores/appsStore';
	import { monitorTask } from '../stores/tasksStore';
	import type { TaskDetail } from '../types';

	export let name = '';
	export let status = 'Stopped';

	let task_id: string = '';
	let failed_task: TaskDetail | null = null;

	function isSupported() {
		return status !== 'Unsupported';
	}

	$: currentIcon = status === 'Running' ? stop : !isSupported() ? unsupported : play;

	async function handleClick() {
		if (!isSupported()) return;
		failed_task = null;
		if (task_id !== '') return;
		task_id = await (status === 'Running' ? stopApp(name) : runApp(name));
		monitorTask(task_id, (result) => {
			if (result.state === 'Failed') {
				failed_task = result;
			}

			task_id = '';
			updateAppInfo(name);
		});
	}
</script>

{#if failed_task}
	<div class="tooltip tooltip-error tooltip-right" data-tip={failed_task.stderr}>
		<button class="btn btn-circle btn-xs bg-error text-white" on:click={handleClick}>
			<Icon icon={errorIcon} />
		</button>
	</div>
{:else}
	<button class="btn btn-circle btn-xs" on:click={handleClick} disabled={!isSupported()}>
		{#if task_id !== ''}
			<span class="loading loading-spinner"></span>
		{:else}
			<Icon icon={currentIcon} />
		{/if}
	</button>
{/if}
