<script lang="ts">
	import { get } from 'svelte/store';
	import { requestTaskDetails, tasks } from '../stores/tasksStore';
	import type { StepState } from '../generated';

	interface Props {
		taskId: string;
		/** Also list the steps and the per-image pull progress. */
		details?: boolean;
	}

	let { taskId, details = false }: Props = $props();

	let progress = $derived($tasks[taskId]?.progress);

	// The task may have started before this page loaded. Fetch it once; a user
	// who may not see the task gets an error and simply sees no bar.
	$effect(() => {
		// get() does not subscribe, so this reruns only when taskId changes.
		if (!get(tasks)[taskId]) requestTaskDetails(taskId).catch(() => {});
	});

	const stepMarks: Record<StepState, string> = {
		pending: '○',
		running: '',
		done: '✓',
		failed: '✗',
		skipped: '–'
	};

	function formatBytes(bytes: number): string {
		if (bytes >= 1e9) return `${(bytes / 1e9).toFixed(1)} GB`;
		if (bytes >= 1e6) return `${(bytes / 1e6).toFixed(1)} MB`;
		if (bytes >= 1e3) return `${(bytes / 1e3).toFixed(0)} kB`;
		return `${bytes} B`;
	}
</script>

{#if progress}
	<div class="flex flex-col gap-1 min-w-40" data-testid="task-progress">
		<progress
			class="progress progress-primary w-full"
			value={Math.round(progress.fraction * 100)}
			max="100"
		></progress>
		<span class="text-xs text-gray-500 truncate" title={progress.status}>{progress.status}</span
		>
		{#if details && progress.steps.length > 0}
			<ul class="text-sm mt-2">
				{#each progress.steps as step, i (i)}
					<li
						class="flex items-center gap-2"
						class:text-gray-400={step.state === 'pending' || step.state === 'skipped'}
						class:text-error={step.state === 'failed'}
						data-state={step.state}
					>
						<span class="w-4 text-center">
							{#if step.state === 'running'}
								<span class="loading loading-spinner loading-xs"></span>
							{:else}
								{stepMarks[step.state]}
							{/if}
						</span>
						{step.name}
					</li>
				{/each}
			</ul>
		{/if}
		{#if details && progress.transfers.length > 0}
			<ul class="text-xs text-gray-500 mt-2 font-mono">
				{#each progress.transfers as transfer (transfer.image)}
					<li>
						{transfer.image}: {transfer.phase}
						{#if transfer.total_bytes > 0}
							{formatBytes(transfer.current_bytes)} / {formatBytes(
								transfer.total_bytes
							)}
						{/if}
						({transfer.layers_done}/{transfer.layers_total} layers)
					</li>
				{/each}
			</ul>
		{/if}
	</div>
{/if}
