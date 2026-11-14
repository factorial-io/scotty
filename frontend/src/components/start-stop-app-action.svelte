<script lang="ts">
	import Icon from '@iconify/svelte';
	import play from '@iconify-icons/ph/play';
	import stop from '@iconify-icons/ph/stop';
	import unsupported from '@iconify-icons/ph/ghost';
	import errorIcon from '@iconify-icons/ph/warning-octagon';
	import { runApp, stopApp } from '../stores/appsStore';
	import { tasks } from '../stores/tasksStore';
	import { hasPermission, permissionsLoaded } from '../stores/permissionStore';
	import type { App } from '../types';
	import { showError } from '../stores/errorStore';

	export let app: App;

	$: name = app.name;
	$: status = app.status;

	// Only covers the gap between the click and the server's response;
	// afterwards `app.operation` says whether the app is busy.
	let pending = false;
	// The last task this button started, to show its failure.
	let last_task_id: string | null = null;
	let dismissed_failure = false;

	function isSupported() {
		return status !== 'Unsupported';
	}

	$: busy = pending || !!app.operation;
	$: failed_task =
		!dismissed_failure && last_task_id && $tasks[last_task_id]?.state === 'Failed'
			? $tasks[last_task_id]
			: null;
	$: canManage = $permissionsLoaded ? hasPermission(app, 'manage') : false;
	$: currentIcon = status === 'Running' ? stop : !isSupported() ? unsupported : play;
	$: isDisabled = !isSupported() || !canManage || busy;

	async function handleClick() {
		if (!isSupported() || !canManage) return;
		dismissed_failure = true;
		if (busy) return;
		pending = true;
		try {
			last_task_id = await (status === 'Running' ? stopApp(name) : runApp(name));
			dismissed_failure = false;
		} catch (err) {
			showError(`${status === 'Running' ? 'Stop' : 'Run'} failed`, err);
		} finally {
			pending = false;
		}
	}
</script>

{#if failed_task}
	<div
		class="tooltip tooltip-error tooltip-right"
		data-tip={`Task failed: ${failed_task.command}. Check task details for output.`}
	>
		<button class="btn btn-circle btn-xs bg-error text-white" on:click={handleClick}>
			<Icon icon={errorIcon} />
		</button>
	</div>
{:else}
	<button
		class="btn btn-circle btn-xs"
		on:click={handleClick}
		disabled={isDisabled}
		aria-busy={busy}
		title={app.operation ? `${app.operation.operation} running` : undefined}
	>
		{#if busy}
			<span class="loading loading-spinner"></span>
		{:else}
			<Icon icon={currentIcon} />
		{/if}
	</button>
{/if}
