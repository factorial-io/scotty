<script lang="ts">
	import Icon from '@iconify/svelte';
	import { get } from 'svelte/store';
	import { requestTaskDetails, tasks } from '../stores/tasksStore';
	import type { StepState } from '../generated';

	interface Props {
		taskId: string;
		/** The app operation, e.g. `rebuild` or `action:drush cr`; leads the progress line. */
		kind?: string;
		/** Offer the steps and the per-image pulls in a collapsed section below the line. */
		details?: boolean;
	}

	let { taskId, kind, details = false }: Props = $props();

	let progress = $derived($tasks[taskId]?.progress);
	// Floor, so the line never says 100 % before the task has finished.
	let percent = $derived(progress ? Math.floor(progress.fraction * 100) : 0);
	let label = $derived(kind?.startsWith('action:') ? kind.slice('action:'.length) : kind);
	let doneSteps = $derived(
		progress
			? progress.steps.filter((s) => s.state === 'done' || s.state === 'skipped').length
			: 0
	);

	// The task may have started before this page loaded. Fetch it once; a user
	// who may not see the task gets an error and simply sees no bar.
	$effect(() => {
		// get() does not subscribe, so this reruns only when taskId changes.
		if (!get(tasks)[taskId]) requestTaskDetails(taskId).catch(() => {});
	});

	const stepIcons: Record<Exclude<StepState, 'running'>, string> = {
		pending: 'ph:circle',
		done: 'ph:check',
		failed: 'ph:x',
		skipped: 'ph:minus'
	};

	function formatBytes(bytes: number): string {
		if (bytes >= 1e9) return `${(bytes / 1e9).toFixed(1)} GB`;
		if (bytes >= 1e6) return `${(bytes / 1e6).toFixed(1)} MB`;
		if (bytes >= 1e3) return `${(bytes / 1e3).toFixed(0)} kB`;
		return `${bytes} B`;
	}
</script>

{#if progress}
	<div class="task-progress" data-testid="task-progress">
		<div class="meta progress-line" title={progress.status}>
			{label ? `${label} · ` : ''}{progress.status} · <span class="mono">{percent} %</span>
		</div>
		<div
			class="progress"
			role="progressbar"
			aria-valuemin="0"
			aria-valuemax="100"
			aria-valuenow={percent}
			aria-label={label ? `${label} progress` : 'Progress'}
		>
			<i style="width: {percent}%"></i>
		</div>
		{#if details && (progress.steps.length > 0 || progress.transfers.length > 0)}
			<details class="progress-details">
				<summary class="meta">
					Show steps ({doneSteps} of {progress.steps.length})
				</summary>
				{#if progress.steps.length > 0}
					<ul class="steps">
						{#each progress.steps as step, i (i)}
							<li
								class:muted={step.state === 'pending' || step.state === 'skipped'}
								class:failed={step.state === 'failed'}
								data-state={step.state}
							>
								{#if step.state === 'running'}
									<Icon icon="ph:spinner" class="ic sm spin" />
								{:else}
									<Icon icon={stepIcons[step.state]} class="ic sm" />
								{/if}
								{step.name}
							</li>
						{/each}
					</ul>
				{/if}
				{#if progress.transfers.length > 0}
					<ul class="transfers mono">
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
			</details>
		{/if}
	</div>
{/if}

<style>
	.task-progress {
		display: flex;
		flex-direction: column;
		gap: 2px;
		min-width: 10rem;
	}
	.progress-line {
		white-space: nowrap;
		overflow: hidden;
		text-overflow: ellipsis;
	}
	.progress-details {
		margin-top: 8px;
	}
	.progress-details summary {
		cursor: pointer;
	}
	.steps,
	.transfers {
		list-style: none;
		margin: 8px 0 0;
		padding: 0;
		display: flex;
		flex-direction: column;
		gap: 4px;
	}
	.steps li {
		display: flex;
		align-items: center;
		gap: 8px;
		font: var(--text-app-body-sm);
	}
	.steps li.muted {
		color: var(--color-copy-secondary);
	}
	.steps li.failed {
		color: var(--color-status-error);
	}
	.transfers {
		color: var(--color-copy-secondary);
	}
</style>
